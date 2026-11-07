//! physis-core CLI.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use physis_core::classify::CellScore;
use physis_core::contradiction::{Contradiction, ContradictionParty, ResolutionStatus};
use physis_core::core::PhysisCore;
use physis_core::embed::VectorEmbed;
use physis_core::history::import_file as import_history_file;
use physis_core::hypothesis::{
    Evidence, EvidencePolarity, Hypothesis, HypothesisStatus, Prediction,
};
use physis_core::models::{
    cosine_sim, Abstraction, Agency, FacetFilter, LifecyclePhase, PinEdit, Scale,
};
use physis_core::ontology::OntologyLoader;
use physis_core::praxis::parse_export as parse_praxis_export;
use physis_core::quality::QualityTracker;
use physis_core::rag::{count_tokens, Bm25Index, RagChunk, RagCorpus, TokenFixedRetriever};
use physis_core::store;
use physis_core::vault::{collect_labels as collect_vault_labels, scan_git_log, scan_vault};
use sha2::{Digest, Sha256};

/// physis-core — embed, classify, cohere, learn from feedback.
#[derive(Parser)]
#[command(name = "physis-core", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Classify text against the semiotic grid.
    Classify { text: String },
    /// Show ontology stats.
    Ontology,
    /// Show the adaptive semiotic grid (axes, shape, populated cells).
    Grid {
        /// Machine-readable output
        #[arg(long)]
        json: bool,
        /// Include every populated cell with its entry names (large)
        #[arg(long)]
        cells: bool,
    },
    /// Query ontology entries by orthogonal facets.
    Facet {
        /// Filter by lifecycle phase: DESIGN | FABRICATE | OPERATE | MAINTAIN | RETIRE
        #[arg(long)]
        lifecycle: Option<String>,
        /// Filter by agency: SELF | OTHER | AUTOMATED | COLLECTIVE
        #[arg(long)]
        agency: Option<String>,
        /// Filter by scale: PERSONAL | INTERPERSONAL | ORGANIZATIONAL | CIVIL
        #[arg(long)]
        scale: Option<String>,
        /// Filter by abstraction: CONCRETE | ABSTRACT
        #[arg(long)]
        abstraction: Option<String>,
        /// Sub-domain (case-insensitive substring)
        #[arg(long)]
        sub_domain: Option<String>,
        /// Sub-mode (case-insensitive substring)
        #[arg(long)]
        sub_mode: Option<String>,
        /// Filter by kind
        #[arg(long)]
        kind: Option<String>,
        /// Filter by domain
        #[arg(long)]
        domain: Option<String>,
        /// Filter by mode
        #[arg(long)]
        mode: Option<String>,
    },
    /// Scan a directory, registering each text file as a coherence node.
    Scan {
        /// Directory to scan recursively.
        dir: PathBuf,
    },
    /// Search recalled coherence nodes by similarity to a query.
    Search {
        /// Query text.
        query: String,
        /// Max results.
        #[arg(long, default_value_t = 10)]
        max: usize,
    },
    /// Edit a coherence node's content ("exchange parts") while preserving structure.
    NodeEdit {
        /// Node ID.
        id: String,
        /// New content (label).
        label: String,
        /// Pin domain (e.g. FABRICATE).
        #[arg(long)]
        domain: Option<String>,
        /// Pin mode (e.g. MAINTAIN).
        #[arg(long)]
        mode: Option<String>,
        /// Clear any existing pin.
        #[arg(long)]
        clear_pin: bool,
    },
    /// Delete a coherence node and its index entry.
    NodeDelete {
        /// Node ID.
        id: String,
    },
    /// Fixed-token budget search over recalled coherence nodes.
    NodeSearch {
        /// Query text.
        query: String,
        /// Token budget for packed results.
        #[arg(long, default_value_t = 600)]
        budget: usize,
        /// Max results.
        #[arg(long, default_value_t = 12)]
        max: usize,
    },
    /// Import a knowledge vault (Obsidian/markdown notes, plain text, git log).
    Vault {
        /// Vault directory to scan.
        dir: PathBuf,
        /// Max git commits to import (0 = skip git).
        #[arg(long, default_value_t = 50)]
        git: usize,
    },
    /// Import personal history (bookmarks HTML, history JSON, OPML, chat JSONL).
    History {
        /// File to import.
        file: PathBuf,
    },
    /// Backfill behavioral records from a Praxis life-log export.
    Praxis {
        /// JSON export file.
        file: PathBuf,
    },
    /// Show a coherence snapshot of the persisted node graph.
    Snapshot,
    /// Report an asserted verdict on a node by label.
    Assert {
        /// Node label (exact).
        label: String,
        /// Verdict: success|inert|failure.
        verdict: String,
    },
    /// Dream over low-coherence / failed nodes.
    Dream,
    /// Quality feedback loop.
    Quality {
        #[command(subcommand)]
        cmd: QualityCmd,
    },
    /// Model registry: list / info / install / remove / path.
    Model {
        #[command(subcommand)]
        cmd: ModelCmd,
    },
    /// N-gram tables: build / list / info / remove / path / inspect / export / import.
    #[command(name = "ngram")]
    NGram {
        #[command(subcommand)]
        cmd: NGramCmd,
    },
    /// Run the pipeline from a JSON config (Directive-2 §16): model / ngram /
    /// corpus / budget are read from the file and overridable on the CLI,
    /// so changing either never requires a source edit.
    Run {
        /// Path to a JSON config file (see `physis-core run --config …`).
        #[arg(long, required = true)]
        config: PathBuf,
        /// Override the configured model registry id.
        #[arg(long)]
        model: Option<String>,
        /// Override the configured ngram registry id (loads it).
        #[arg(long)]
        ngram: Option<String>,
        /// Override the configured query.
        #[arg(long)]
        query: Option<String>,
    },
    /// Reproducible benchmark over a deterministic ground-truth corpus.
    /// Writes machine-readable artifacts to benchmarks/results/ (Directives 1+2).
    Benchmark {
        /// N-gram order for the infra leg.
        #[arg(long, default_value_t = 5)]
        order: u8,
        /// Context budget for the compression leg.
        #[arg(long, default_value_t = 300)]
        budget: usize,
        /// Optional reference (big) model id for the oracle leg — measured
        /// only when an external harness supplies it; otherwise reported as
        /// `not_configured` with no claim made.
        #[arg(long)]
        big_model: Option<String>,
    },
    /// Watch a source and append what changed to the observation log.
    /// Append-only: nothing here is ever rewritten. Re-running over an
    /// unchanged tree produces nothing.
    #[command(name = "watch")]
    Watch {
        /// `fs` — content-hash gated tree scan.
        /// `git` — commits, with the interval since the previous commit.
        /// `terminal` — shell history, with each command's real runtime.
        /// `agent` — an agent's own session turns (model outputs and prompts).
        /// `process` — long-lived processes, with their real age.
        /// `browser` — visited pages (needs the `sqlite3` CLI).
        /// `all` — every source above, in one pass.
        #[arg(long, default_value = "fs")]
        source: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value_t = 2000)]
        max: usize,
        /// Show what would be appended without writing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Read the observation log: what this machine saw, and what else was
    /// happening at the same time.
    #[command(name = "observed")]
    Observed {
        /// Only this bucket (`fs`, `git`, `terminal`, `model`, …).
        #[arg(long)]
        source: Option<String>,
        /// What overlapped this observation in time, across every source.
        #[arg(long)]
        concurrent: Option<u64>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Run a command, record it, and say what is already believed about it.
    ///
    /// Not a sandbox — `sh -c` already runs things. The contribution is that
    /// the intent is recorded BEFORE the command runs (so an action that hangs
    /// still leaves a record), the outcome is recorded with its real duration,
    /// and claims bearing on it are surfaced first — especially ones already
    /// Contradicted, which is the machine saying you established this does not
    /// work.
    #[command(name = "act")]
    Act {
        /// The command, as you would type it into a shell.
        command: String,
        /// Show what is already believed and STOP, without running anything.
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value_t = 5)]
        top: usize,
        #[arg(long)]
        json: bool,
    },
    /// Promote an observation into a claim that can be refuted.
    ///
    /// The observation stays in the log untouched; the claim cites it as
    /// `obs:<seq>` so the evidence can always be re-read rather than trusted.
    /// This is the step from "this was seen" to "this is so, and it might be
    /// wrong" — the only step that lets the machine notice it was mistaken
    /// rather than merely outdated.
    #[command(name = "claim")]
    Claim {
        /// Sequence number from `physis-core observed`.
        #[arg(long)]
        from: u64,
        /// What you assert on the strength of it.
        statement: String,
        #[arg(long, default_value_t = 0.5)]
        confidence: f32,
    },
    /// Did a transformation go the way it was asked to — and did it beat doing
    /// the same thing stupidly? Reads two files and answers with the null in
    /// the same pass.
    #[command(name = "direction")]
    DirectionCmd {
        #[arg(long)]
        before: PathBuf,
        #[arg(long)]
        after: PathBuf,
        /// `shorter` | `longer` | `rephrased`
        #[arg(long, default_value = "shorter")]
        want: String,
        #[arg(long)]
        json: bool,
    },
    /// The conceptual state a human and a machine are both working on: what is
    /// believed and on what evidence, what is still disputed, and what was
    /// predicted and never scored. No model is involved — it reads the store.
    #[command(name = "ground")]
    Ground {
        #[arg(long)]
        json: bool,
    },
    /// The whole engine in one pass: structure, bounded context, coverage
    /// gaps, a shortlist per gap, drift detection — and the label-permuted
    /// control that all of it is scored against.
    #[command(name = "chain")]
    Chain {
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long, default_value_t = 1200)]
        budget: usize,
        /// Cosine below which a record counts as unplaced.
        #[arg(long, default_value_t = 0.75)]
        threshold: f32,
        #[arg(long)]
        json: bool,
    },
    /// Grounded answer over a local corpus, with citations (the "NotebookLM"
    /// shape). Synthesises when an OpenAI-compatible endpoint is configured
    /// (a local ollama counts); otherwise returns a labelled extract and says
    /// why. See the `notebook` module docs for the two tiers.
    #[command(name = "notebook")]
    Notebook {
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 1200)]
        budget: usize,
        #[arg(long)]
        json: bool,
        /// Draft-and-fill: let the table draft what the retrieved context
        /// supports and mark the rest as gaps, instead of asking a model for
        /// the whole answer. Prints `table_share` — the fraction the table
        /// supplied, which is the number that says whether this approach
        /// applies to the question at all.
        #[arg(long)]
        draft: bool,
        /// Probability floor below which the table declines to continue and
        /// opens a gap. Higher means shorter, safer drafts.
        #[arg(long, default_value_t = 0.0)]
        confidence: f32,
    },
    /// Physis context compiler: fixed-budget structural context for a query
    /// against a corpus, with the measured compression over conventional
    /// retrieval (Demo B).
    #[command(name = "context")]
    Context {
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 1200)]
        budget: usize,
        #[arg(long)]
        json: bool,
    },
    /// One-command offline demo: corpus → map → ngram table → model answer.
    Demo {
        /// Corpus directory (any text file; binaries and build dirs are skipped).
        #[arg(long, default_value = "examples")]
        dir: PathBuf,
        /// Query whose continuation the local model completes.
        #[arg(long, default_value = "the pump")]
        query: String,
        /// N-gram order for the table built on the fly.
        #[arg(long, default_value_t = 5)]
        order: u8,
    },
    /// Epistemic hypothesis management.
    Hypothesis {
        #[command(subcommand)]
        cmd: HypothesisCmd,
    },
    /// Contradiction management and truth maintenance.
    Contradiction {
        #[command(subcommand)]
        cmd: ContradictionCmd,
    },
    /// View chronological epistemic audit log.
    Audit,
    /// Replay historical belief state for a subject at timestamp T.
    Replay {
        /// Subject ID or hypothesis ID.
        #[arg(long)]
        subject: String,
        /// Optional ISO 8601 timestamp.
        #[arg(long)]
        at: Option<String>,
    },
    /// Discover ontology gaps and candidate domains from unstructured corpus texts.
    Discover {
        /// Directory containing text files, or scan persisted nodes if omitted.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Minimum cluster size for domain proposal.
        #[arg(long, default_value_t = 3)]
        min_cluster: usize,
    },
    /// Run the ontology studio web GUI.
    #[cfg(feature = "studio")]
    Studio {
        /// Port to listen on.
        #[arg(long, default_value_t = 3000)]
        port: u16,
        /// ONNX model directory (model.onnx + tokenizer.json). Optional — falls
        /// back to deterministic random projection when absent or unloadable.
        #[arg(long)]
        model: Option<String>,
    },
}

#[derive(Subcommand)]
enum QualityCmd {
    /// Show quality tracker summary.
    Summary,
    /// Report a failure with optional correct domain.
    Fail { feedback: String },
    /// Report a success for a cell (DOMAIN\x00MODE).
    Pass { cell: String },
}

#[derive(Subcommand)]
enum HypothesisCmd {
    /// List all registered hypotheses.
    List,
    /// Register a candidate hypothesis.
    Create {
        /// Statement of the hypothesis.
        statement: String,
        /// Initial confidence [0.0, 1.0].
        #[arg(long, default_value_t = 0.5)]
        confidence: f32,
    },
    /// Add supporting or contradicting evidence to a hypothesis.
    Evidence {
        /// Hypothesis ID.
        id: String,
        /// Evidence claim description.
        claim: String,
        /// Source identifier.
        #[arg(long, default_value = "cli")]
        source: String,
        /// Polarity: supporting (default) or contradicting.
        #[arg(long, default_value = "supporting")]
        polarity: String,
        /// Evidence weight [0.0, 1.0].
        #[arg(long, default_value_t = 0.8)]
        weight: f32,
    },
    /// Record a testable prediction on a hypothesis.
    Predict {
        /// Hypothesis ID.
        id: String,
        /// Prediction statement.
        statement: String,
        /// Expected outcome.
        #[arg(long)]
        expected: Option<String>,
    },
    /// Transition hypothesis lifecycle status.
    Transition {
        /// Hypothesis ID.
        id: String,
        /// New status: Candidate|Supported|Contradicted|Confirmed|Inert|Failed|Superseded|Isolated|Certified
        status: String,
        /// Reason for transition.
        #[arg(long, default_value = "cli transition")]
        reason: String,
    },
    /// Show structured explanation report for a hypothesis.
    Explain {
        /// Hypothesis ID (full UUID or any unambiguous prefix).
        id: String,
    },
    /// Rank registered hypotheses by relevance to a topic.
    ///
    /// This is the retrieval step of the development loop. `list` prints
    /// everything in insertion order, which stops being readable at ~30 claims
    /// and so quietly stops being run.
    Query {
        /// Topic or question to retrieve against.
        topic: String,
        /// Max results.
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Include hypotheses whose epistemic status is dead (Failed, Inert,
        /// Superseded). Off by default: they are the answer to "what was tried"
        /// and noise for "what is believed".
        #[arg(long)]
        include_dead: bool,
    },
    /// Record what actually happened to a prediction, and let fitness move.
    Resolve {
        /// Hypothesis ID (full UUID or any unambiguous prefix).
        id: String,
        /// Which prediction, by index as shown by `open` or `explain`.
        index: usize,
        /// What actually happened.
        actual: String,
        /// The prediction held.
        #[arg(long, conflicts_with = "wrong")]
        correct: bool,
        /// The prediction did not hold.
        #[arg(long)]
        wrong: bool,
    },
    /// List predictions that were made and never resolved.
    ///
    /// The point of the exercise: it makes you confront what you predicted and
    /// never checked.
    Open {
        /// Only show predictions older than this many days.
        #[arg(long, default_value_t = 0)]
        older_than: i64,
    },
}

#[derive(Subcommand)]
enum ContradictionCmd {
    /// List all active and resolved contradictions.
    List,
    /// Record a contradiction between two conflicting claims.
    Record {
        /// First claim statement.
        claim_a: String,
        /// Source of first claim.
        source_a: String,
        /// Second claim statement.
        claim_b: String,
        /// Source of second claim.
        source_b: String,
    },
    /// Resolve a contradiction.
    Resolve {
        /// Contradiction ID.
        id: String,
        /// Resolution status: APreferred|BPreferred|Open|Contextual|Superseded
        status: String,
        /// Explanation for resolution.
        #[arg(long)]
        explanation: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    store::ensure_data_dir()?;
    match cli.command {
        Command::Classify { text } => run_classify(&text),
        Command::Ontology => run_ontology(),
        Command::Grid { json, cells } => run_grid(json, cells),
        Command::Facet {
            lifecycle,
            agency,
            scale,
            abstraction,
            sub_domain,
            sub_mode,
            kind,
            domain,
            mode,
        } => run_facet(
            lifecycle,
            agency,
            scale,
            abstraction,
            sub_domain,
            sub_mode,
            kind,
            domain,
            mode,
        ),
        Command::Scan { dir } => run_scan(&dir),
        Command::Search { query, max } => run_search(&query, max),
        Command::NodeEdit {
            id,
            label,
            domain,
            mode,
            clear_pin,
        } => run_node_edit(&id, &label, domain, mode, clear_pin),
        Command::NodeDelete { id } => run_node_delete(&id),
        Command::NodeSearch { query, budget, max } => run_node_search(&query, budget, max),
        Command::Vault { dir, git } => run_vault(&dir, git),
        Command::History { file } => run_history(&file),
        Command::Praxis { file } => run_praxis(&file),
        Command::Snapshot => run_snapshot(),
        Command::Assert { label, verdict } => run_assert(&label, &verdict),
        Command::Dream => run_dream(),
        Command::Quality { cmd } => run_quality(cmd),
        Command::Hypothesis { cmd } => run_hypothesis(cmd),
        Command::Contradiction { cmd } => run_contradiction(cmd),
        Command::Audit => run_audit(),
        Command::Replay { subject, at } => run_replay(&subject, at.as_deref()),
        Command::Discover { dir, min_cluster } => run_discover(dir.as_deref(), min_cluster),
        Command::Model { cmd } => cmd_model(cmd),
        Command::NGram { cmd } => cmd_ngram(cmd),
        Command::Demo { dir, query, order } => cmd_demo(&dir, &query, order),
        Command::Watch {
            source,
            path,
            max,
            dry_run,
        } => cmd_watch(&source, &path, max, dry_run),
        Command::Observed {
            source,
            concurrent,
            limit,
            json,
        } => cmd_observed(source.as_deref(), concurrent, limit, json),
        Command::Act {
            command,
            dry_run,
            top,
            json,
        } => cmd_act(&command, dry_run, top, json),
        Command::Claim {
            from,
            statement,
            confidence,
        } => cmd_claim(from, &statement, confidence),
        Command::DirectionCmd {
            before,
            after,
            want,
            json,
        } => cmd_direction(&before, &after, &want, json),
        Command::Ground { json } => {
            let mut core = load_core();
            if core.project_all_revisions() > 0 {
                store::ensure_data_dir()?;
                core.persist()?;
            }
            let g = physis_core::ground::read(
                &core,
                &physis_core::observe::read(&physis_core::observe::log_path()).unwrap_or_default(),
                chrono::Utc::now(),
            );
            if json {
                println!("{}", serde_json::to_string_pretty(&g)?);
            } else {
                print!("{}", g.render());
            }
            Ok(())
        }
        Command::Chain {
            corpus,
            query,
            budget,
            threshold,
            json,
        } => cmd_chain(&corpus, &query, budget, threshold, json),
        Command::Notebook {
            corpus,
            query,
            budget,
            json,
            draft,
            confidence,
        } => cmd_notebook(&corpus, &query, budget, json, draft, confidence),
        Command::Context {
            corpus,
            query,
            budget,
            json,
        } => cmd_context(&corpus, &query, budget, json),
        Command::Benchmark {
            order,
            budget,
            big_model,
        } => cmd_benchmark(order, budget, big_model),
        Command::Run {
            config,
            model,
            ngram,
            query,
        } => cmd_run(&config, model, ngram, query),
        #[cfg(feature = "studio")]
        Command::Studio { port, model } => run_studio(port, model),
    }
}

/// Resolve the CLI's embedder through the self-test-gated cascade.
///
/// Was `RandomProjectionEmbedder::new(384)` unconditionally, which meant every
/// number this CLI (and the benchmark it drives) ever printed came from a
/// lexical hash. `embed::select` picks a real model when one is present and
/// passes the semantic probe, and falls back to random projection *labelled as
/// such* when it is not.
fn load_embedder() -> Box<dyn physis_core::embed::VectorEmbed> {
    load_embedder_kind().0
}

/// The embedder plus the label describing what actually loaded.
fn load_embedder_kind() -> (Box<dyn physis_core::embed::VectorEmbed>, &'static str) {
    let (e, kind) = physis_core::embed::select(384);
    if kind == "random-projection" {
        eprintln!(
            "warning: embedder is random-projection (non-semantic). \
Similarity here is lexical, not meaning-based. Put a model in \
~/.physis/models/minilm (model.onnx + tokenizer.json) or set PHYSIS_MODEL_DIR."
        );
    }
    (e, kind)
}

/// Build the classifier through the on-disk embedding cache.
///
/// Embedding ~760 ontology entries costs seconds on CPU; the CLI is invoked as
/// a subprocess, so without a cache every classify/grid/context call paid it
/// again. Keyed by model + ontology text, so upgrades invalidate it.
fn build_classifier(
    ontology: &OntologyLoader,
    embedder: &dyn physis_core::embed::VectorEmbed,
    kind: &str,
) -> physis_core::classify::CellClassifier {
    let cache_dir = std::env::var("PHYSIS_CACHE_DIR")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| physis_core::store::data_dir().join("cache"));
    // Stable across installs: the absolute model path changes with the
    // plugin directory, which would make a shipped warm cache useless.
    let model_dir = std::env::var("PHYSIS_MODEL_DIR").unwrap_or_default();
    let model_identity = if model_dir.trim().is_empty() {
        "default".to_string()
    } else {
        let p = std::path::Path::new(&model_dir);
        let name = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("model");
        let size = std::fs::metadata(p.join("model.onnx"))
            .map(|m| m.len())
            .unwrap_or(0);
        format!("{name}|{size}")
    };
    let model_key = format!("{kind}|{}d|{model_identity}", embedder.dimension());
    physis_core::classify::CellClassifier::build_cached(ontology, embedder, &cache_dir, &model_key)
}

fn load_quality() -> QualityTracker {
    QualityTracker::load_or_new(&store::quality_path())
}

fn run_classify(text: &str) -> anyhow::Result<()> {
    let ontology = OntologyLoader::load_all();
    let (embedder, embedder_kind) = load_embedder_kind();
    let classifier = build_classifier(&ontology, embedder.as_ref(), embedder_kind);
    let quality = load_quality();

    let results = classifier.classify_text(text, &embedder);
    let adjusted: Vec<CellScore> = results
        .iter()
        .map(|r| {
            let key = format!("{}\x00{}", r.domain, r.mode);
            let mut r2 = r.clone();
            r2.score = quality.adjust_score(&key, r.score);
            r2
        })
        .collect();

    println!("Query: {text}");
    let grid = physis_core::GridView::from_classifier(&classifier);
    println!(
        "Grid: {}×{} = {} cells ({} populated, {} entries){}",
        grid.shape[0],
        grid.shape[1],
        grid.total_cells,
        grid.populated_cells,
        grid.entries,
        if grid.is_expanded() {
            " — expanded beyond 5×14 by the loaded ontologies"
        } else {
            ""
        }
    );

    // Scores cluster in the top few percent, so a bar drawn against an absolute
    // 0..1 scale saturates and paints every row a full block — the strongest and
    // the eighth-best look equally certain. Draw the bar over the spread of the
    // rows actually shown, which is where the discriminating information is.
    let shown: Vec<&CellScore> = adjusted.iter().take(8).collect();
    let hi = shown.first().map(|r| r.score).unwrap_or(0.0);
    let lo = shown.last().map(|r| r.score).unwrap_or(0.0);
    let span = hi - lo;
    for r in &shown {
        let filled = if span > f32::EPSILON {
            (((r.score - lo) / span) * 20.0).round() as usize
        } else {
            20
        };
        println!(
            "  {:<11} × {:<10} {:5.3} {}{}",
            r.domain,
            r.mode,
            r.score,
            "█".repeat(filled),
            "░".repeat(20 - filled)
        );
    }
    println!(
        "  (bars are relative to the {} shown, not absolute)",
        shown.len()
    );

    // The gap to the runner-up is the confidence signal. Without it a reader has
    // to subtract two numbers to notice the engine effectively did not choose.
    if let (Some(first), Some(second)) = (adjusted.first(), adjusted.get(1)) {
        let margin = first.score - second.score;
        let verdict = if margin < 0.02 {
            "  ⚠ effectively a tie — treat the top cell as unresolved"
        } else if margin < 0.08 {
            "  · weak separation"
        } else {
            ""
        };
        println!("Margin over runner-up: {margin:.3}{verdict}");
    }

    if let Some((sim, domain, mode)) = classifier.best_entry_sim(&embedder.embed(text)) {
        println!("Best entry cosine: {sim:.3} ({domain}×{mode})");
    }
    let semantic = embedder_kind != "random-projection";
    println!(
        "Embedder: {} ({}d) — {}.",
        embedder_kind,
        embedder.dimension(),
        if semantic {
            "semantic model; similarity is meaning-based"
        } else {
            "deterministic feature hashing, reproducible but NOT semantic"
        }
    );
    Ok(())
}

/// Print the adaptive grid: every distinct domain/mode the ontologies define.
fn run_grid(as_json: bool, include_cells: bool) -> anyhow::Result<()> {
    let ontology = OntologyLoader::load_all();
    let (embedder, kind) = load_embedder_kind();
    let classifier = build_classifier(&ontology, embedder.as_ref(), kind);
    let grid = physis_core::GridView::from_classifier(&classifier);

    if as_json {
        let mut payload = serde_json::json!({
            "domains": grid.domains,
            "modes": grid.modes,
            "shape": grid.shape,
            "total_cells": grid.total_cells,
            "populated_cells": grid.populated_cells,
            "entries": grid.entries,
            "expanded": grid.is_expanded(),
            "embedder": {"kind": kind, "dimension": embedder.dimension()},
        });
        if include_cells {
            payload["cells"] = serde_json::json!(grid.cells);
        }
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!(
        "Semiotic grid: {} domains × {} modes = {} cells ({} populated, {} entries)",
        grid.shape[0], grid.shape[1], grid.total_cells, grid.populated_cells, grid.entries
    );
    if grid.is_expanded() {
        println!("Expanded beyond the classic 5×14: the axes are whatever the loaded ontologies define.");
    }
    println!("Domains: {}", grid.domains.join(", "));
    println!("Modes: {}", grid.modes.join(", "));
    println!("Populated cells:");
    for cell in &grid.cells {
        println!(
            "  {:<11} × {:<10} {:>3} entries",
            cell.domain,
            cell.mode,
            cell.entries.len()
        );
    }
    println!("Embedder: {} ({}d)", kind, embedder.dimension());
    Ok(())
}

fn run_ontology() -> anyhow::Result<()> {
    let ontology = OntologyLoader::load_all();
    println!(
        "Ontology: {} classification entries",
        ontology.entry_count()
    );
    println!("  human (grid) domains: {}", ontology.human_domains.len());
    println!(
        "  custom (extra) domains: {}",
        ontology.custom_domains.len()
    );
    println!("Categories:");
    for cat in ontology.categories() {
        let n = ontology
            .all_domains()
            .values()
            .filter(|d| d.category.as_deref() == Some(cat.as_str()))
            .count();
        println!("  {cat}: {n}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_facet(
    lifecycle: Option<String>,
    agency: Option<String>,
    scale: Option<String>,
    abstraction: Option<String>,
    sub_domain: Option<String>,
    sub_mode: Option<String>,
    kind: Option<String>,
    domain: Option<String>,
    mode: Option<String>,
) -> anyhow::Result<()> {
    let ontology = OntologyLoader::load_all();
    let filter = FacetFilter {
        lifecycle: lifecycle.as_deref().and_then(LifecyclePhase::parse_facet),
        agency: agency.as_deref().and_then(Agency::parse_facet),
        scale: scale.as_deref().and_then(Scale::parse_facet),
        abstraction: abstraction.as_deref().and_then(Abstraction::parse_facet),
        sub_domain,
        sub_mode,
        kind,
        domain,
        mode,
    };

    let mut matches = Vec::new();
    for (name, entry) in ontology.all_domains() {
        if let Some(ref k) = filter.kind {
            if entry.axis_kind.as_deref() != Some(k.as_str()) {
                continue;
            }
        }
        if let Some(ref d) = filter.domain {
            if !entry
                .domain
                .as_deref()
                .unwrap_or_default()
                .eq_ignore_ascii_case(d)
            {
                continue;
            }
        }
        if let Some(ref m) = filter.mode {
            if !entry
                .mode
                .as_deref()
                .unwrap_or_default()
                .eq_ignore_ascii_case(m)
            {
                continue;
            }
        }
        if entry.facets.matches(&filter) {
            matches.push((name, entry));
        }
    }

    println!("Facet filter matched {} entries:", matches.len());
    for (name, entry) in matches.iter().take(40) {
        println!(
            "  {:<32} {:<10} × {:<10} [{}]",
            name,
            entry.domain.as_deref().unwrap_or("—"),
            entry.mode.as_deref().unwrap_or("—"),
            entry.facets.simple()
        );
    }
    if matches.len() > 40 {
        println!("  ... and {} more", matches.len() - 40);
    }
    Ok(())
}

fn run_scan(dir: &std::path::Path) -> anyhow::Result<()> {
    let embedder = load_embedder();
    let mut core = load_core();
    let mut count = 0usize;
    for entry in walk(dir)? {
        let Ok(text) = std::fs::read_to_string(&entry) else {
            continue;
        };
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 5000 {
            continue;
        }
        core.register_node_from_text(&format!("{}: {}", entry.display(), trimmed), &embedder);
        count += 1;
    }
    core.persist()?;
    let snap = core.snapshot();
    println!("Registered {count} files. Nodes: {}", snap.total_nodes);
    println!("Coherence index: {:.3}", snap.coherence_index);
    Ok(())
}

fn run_search(query: &str, max: usize) -> anyhow::Result<()> {
    let embedder = load_embedder();
    let core = load_core();
    let hits = core.search_text(query, &embedder, max);
    if hits.is_empty() {
        println!("No recalled nodes. Try `physis-core scan <dir>` first.");
        return Ok(());
    }
    // `max` is the cap asked for, not what was found — printing it as the count
    // claimed "Top 10 matches" over three results.
    println!("{} match(es) for: {}", hits.len(), query);
    for (i, (_, label, score)) in hits.iter().enumerate() {
        println!(
            "  {}. {:.3}  {}",
            i + 1,
            score,
            elide_middle(&single_line(label), 110)
        );
    }
    Ok(())
}

/// Collapse a node label onto one line so a list stays a list.
///
/// Labels carry file content verbatim, newlines and all, and printing them raw
/// made a numbered row spill across lines with no indentation — the rows after
/// the first read as separate results.
fn single_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Shorten `s` to `max` characters by dropping the MIDDLE, not the tail.
///
/// Node labels are `"<absolute path>: <content>"`, so truncating from the front
/// spends the whole budget on the shared leading directory and every hit prints
/// as the same string. Keeping both ends preserves the two parts that actually
/// identify a node — the file name and the start of its text.
fn elide_middle(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    let head = max / 3;
    let tail_budget = max.saturating_sub(head + 1);

    // Resume at the last path separator when the file name still fits, so the
    // cut never lands inside it — "…port_049.txt" and "…port_050.txt" are two
    // characters apart and read as the same node at a glance. Content is cut at
    // its end instead, which loses nothing that identifies the node.
    let resume = chars.iter().rposition(|c| *c == '/').map(|i| i + 1);
    let tail: String = match resume {
        Some(i) if chars.len() - i > tail_budget => chars[i..i + tail_budget].iter().collect(),
        _ => chars[chars.len() - tail_budget..].iter().collect(),
    };

    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.push_str(&tail);
    out
}

fn run_node_edit(
    id: &str,
    label: &str,
    domain: Option<String>,
    mode: Option<String>,
    clear_pin: bool,
) -> anyhow::Result<()> {
    let embedder = load_embedder();
    let mut core = load_core();
    let new_emb = embedder.embed(label);
    let pin = if clear_pin {
        PinEdit::Clear
    } else if let (Some(d), Some(m)) = (domain, mode) {
        PinEdit::Set(d, m)
    } else {
        PinEdit::Keep
    };
    let outcome = core.edit_node(id, label, new_emb, pin, Some("random-projection"))?;
    core.persist()?;
    println!(
        "Node {} updated → '{}' (asserted {:?}, coherence {:.3})",
        outcome.node_id, outcome.label, outcome.asserted, outcome.coherence_score
    );
    if let Some((d, m)) = outcome.pinned_cell {
        println!("  pinned cell: {}×{}", d, m);
    }
    Ok(())
}

fn run_node_delete(id: &str) -> anyhow::Result<()> {
    let mut core = load_core();
    if core.delete_node(id) {
        core.persist()?;
        println!("Node {} deleted", id);
    } else {
        println!("Node {} not found", id);
    }
    Ok(())
}

fn run_node_search(query: &str, budget: usize, max: usize) -> anyhow::Result<()> {
    let embedder = load_embedder();
    let core = load_core();
    let q_emb = embedder.embed(query);
    let candidates: Vec<(usize, String, Vec<f32>)> = core
        .nodes
        .values()
        .filter_map(|n| {
            let l = n.label.as_deref()?;
            if l.trim().is_empty() {
                return None;
            }
            Some((l.to_string(), n.embedding.clone()))
        })
        .enumerate()
        .map(|(i, (l, e))| (i, l, e))
        .collect();

    let corpus = RagCorpus {
        chunks: candidates
            .iter()
            .map(|(i, l, e)| RagChunk {
                id: *i,
                text: l.clone(),
                tokens: count_tokens(l),
                embedding: e.clone(),
            })
            .collect(),
        bm25: Bm25Index::build(
            &candidates
                .iter()
                .map(|(_, l, _)| l.clone())
                .collect::<Vec<String>>(),
        ),
    };
    let result = TokenFixedRetriever::new(budget, max).retrieve(&q_emb, &corpus);
    println!(
        "Fixed-token search: {} results ({} tokens, budget {})",
        result.chunks.len(),
        result.total_tokens,
        result.budget
    );
    for (i, c) in result.chunks.iter().enumerate() {
        println!(
            "  {}. {:.3}  {} ({} tokens)",
            i + 1,
            c.score,
            c.text,
            c.tokens
        );
    }
    Ok(())
}

fn run_vault(dir: &std::path::Path, git: usize) -> anyhow::Result<()> {
    let embedder = load_embedder();
    let mut core = load_core();
    let mut docs = scan_vault(dir);
    if git > 0 {
        let commits = scan_git_log(dir, git);
        docs.extend(commits);
    }
    if docs.is_empty() {
        println!("No docs found in vault: {}", dir.display());
        return Ok(());
    }
    let pairs = collect_vault_labels(&docs);
    let before = core.nodes.len();
    for (label, text) in &pairs {
        let emb = embedder.embed(text);
        core.register_node_vec_labeled(emb, Some(label.clone()));
    }
    let added = core.nodes.len().saturating_sub(before);
    core.persist()?;
    println!(
        "Vault[{}]: {} docs → {} labels registered ({} deduped)",
        dir.display(),
        docs.len(),
        added,
        pairs.len().saturating_sub(added)
    );
    Ok(())
}

fn run_history(file: &std::path::Path) -> anyhow::Result<()> {
    let embedder = load_embedder();
    let mut core = load_core();
    let (docs, name) = import_history_file(file)?;
    let before = core.nodes.len();
    for doc in &docs {
        let emb = embedder.embed(&doc.body);
        core.register_node_vec_labeled(emb, Some(doc.title.clone()));
    }
    let added = core.nodes.len().saturating_sub(before);
    core.persist()?;
    println!(
        "History[{}]: {} items → {} nodes registered",
        name,
        docs.len(),
        added
    );
    Ok(())
}

fn run_praxis(file: &std::path::Path) -> anyhow::Result<()> {
    let embedder = load_embedder();
    let mut core = load_core();
    let text = std::fs::read_to_string(file)?;
    let records = parse_praxis_export(&text);
    let before = core.nodes.len();
    for r in &records {
        let emb = embedder.embed(&r.body);
        let id =
            core.register_node_vec_labeled(emb, Some(format!("praxis:{}:{}", r.kind, r.title)));
        core.assert_coherence(&id, r.status.as_score());
    }
    let added = core.nodes.len().saturating_sub(before);
    core.persist()?;
    println!(
        "Praxis[{}]: {} records → {} behavioral nodes asserted",
        file.display(),
        records.len(),
        added
    );
    Ok(())
}

fn run_snapshot() -> anyhow::Result<()> {
    let core = load_core();
    let snap = core.snapshot();
    println!("Coherence snapshot:");
    println!("  nodes:            {}", snap.total_nodes);
    println!("  coherence index:  {:.3}", snap.coherence_index);
    println!(
        "  high/mid/low:     {}/{}/{}",
        snap.high_coherence, snap.mid_coherence, snap.low_coherence
    );
    println!("  certified branches: {}", snap.certified_branches_count);
    println!("  isolated branches:  {}", snap.isolated_branches_count);
    println!(
        "  asserted s/i/f:   {}/{}/{}",
        snap.asserted_success, snap.asserted_inert, snap.asserted_failure
    );
    println!("  asserted index:   {:?}", snap.asserted_index);
    Ok(())
}

fn run_assert(label: &str, verdict: &str) -> anyhow::Result<()> {
    let mut core = load_core();
    let id = core
        .nodes
        .values()
        .find(|n| n.label.as_deref() == Some(label))
        .map(|n| n.id.clone())
        .ok_or_else(|| anyhow::anyhow!("no node with label '{label}'"))?;
    let score = match verdict.to_lowercase().as_str() {
        "success" => 1.0,
        "inert" => 0.0,
        "failure" => -1.0,
        other => anyhow::bail!("verdict must be success|inert|failure, got '{other}'"),
    };
    core.assert_coherence(&id, score);
    core.persist()?;
    println!("Asserted '{label}' as {verdict}");
    Ok(())
}

fn run_dream() -> anyhow::Result<()> {
    let mut core = load_core();
    let results = core.dream();
    if results.is_empty() {
        println!("No nodes need replaying right now.");
        return Ok(());
    }
    for d in &results {
        println!(
            "Dream {} tested {} node(s): outcome {:.1} prevented_failure={}",
            &d.dream_id[..8],
            d.nodes_tested.len(),
            d.outcome,
            d.prevented_failure,
        );
    }
    core.persist()?;
    Ok(())
}

fn run_quality(cmd: QualityCmd) -> anyhow::Result<()> {
    let mut quality = load_quality();
    match cmd {
        QualityCmd::Summary => {
            println!("Quality tracker: {} failures", quality.failures.len());
            println!("  cells penalized: {}", quality.cell_penalties.len());
            println!("  cells boosted:   {}", quality.cell_boosts.len());
            let mut penalties: Vec<_> = quality.cell_penalties.iter().collect();
            penalties.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
            for (cell, p) in penalties.iter().take(10) {
                println!("  {} penalty={:.2}", cell.replace('\x00', " × "), p);
            }
            for f in quality.failures.iter().rev().take(5) {
                println!(
                    "  [{}] {} score={:.3} sev={:.1} feedback={}",
                    &f.id[..8],
                    f.top_cell.replace('\x00', " × "),
                    f.top_score,
                    f.severity,
                    f.feedback.chars().take(50).collect::<String>(),
                );
            }
        }
        QualityCmd::Fail { feedback } => {
            let ontology = OntologyLoader::load_all();
            let (embedder, kind) = load_embedder_kind();
            let classifier = build_classifier(&ontology, embedder.as_ref(), kind);
            let centroids = classifier.cell_centroids();
            let f = quality.report_failure(&feedback, &centroids, None);
            println!(
                "Recorded failure → {} (score {:.3})",
                f.top_cell.replace('\x00', " × "),
                f.top_score
            );
            quality.save(&store::quality_path())?;
        }
        QualityCmd::Pass { cell } => {
            quality.report_success(&cell);
            quality.save(&store::quality_path())?;
            println!("Boosted cell '{}'", cell.replace('\x00', " × "));
        }
    }
    Ok(())
}

/// `IdMatch` as a `Result`, for the mutating subcommands.
///
/// Prefix resolution already existed here — but only `replay` used it. Every
/// mutating subcommand did a `HashMap::get_mut` on the full 36-character UUID
/// while `hypothesis list` printed `&h.id[..8]`, so nothing the tool printed
/// could be pasted back into it: `hypothesis predict 7824e0f2 ...` answered
/// "not found". Friction like that does not make a loop slower, it makes it
/// stop running. Ambiguity is an error that names the candidates rather than
/// picking one.
fn require_hypothesis_id(core: &PhysisCore, needle: &str) -> anyhow::Result<String> {
    match resolve_hypothesis_id(core, needle) {
        IdMatch::Exact(id) => Ok(id),
        IdMatch::Unknown => anyhow::bail!("hypothesis {needle} not found"),
        IdMatch::Ambiguous(_) => {
            let mut msg = format!("'{needle}' is ambiguous — these hypotheses match:\n");
            for (id, h) in core
                .hypotheses
                .iter()
                .filter(|(id, _)| id.starts_with(needle))
            {
                msg.push_str(&format!("  {id}  {}\n", h.statement));
            }
            msg.push_str("Use more characters of the id.");
            anyhow::bail!(msg)
        }
    }
}

/// Statuses that mean "this claim is no longer a live belief".
fn is_dead(status: HypothesisStatus) -> bool {
    matches!(
        status,
        HypothesisStatus::Failed | HypothesisStatus::Inert | HypothesisStatus::Superseded
    )
}

fn run_hypothesis(cmd: HypothesisCmd) -> anyhow::Result<()> {
    let mut core = load_core();
    let embedder = load_embedder();
    match cmd {
        HypothesisCmd::List => {
            println!("Registered hypotheses: {}", core.hypotheses.len());
            for h in core.hypotheses.values() {
                println!(
                    "  [{}] ({:?}) fitness={:.3} — {}",
                    &h.id[..8],
                    h.status,
                    h.fitness,
                    h.statement
                );
            }
        }
        HypothesisCmd::Create {
            statement,
            confidence,
        } => {
            let emb = embedder.embed(&statement);
            let mut hyp = Hypothesis::new(&statement, emb);
            hyp.confidence = confidence;
            let id = core.register_hypothesis(hyp);
            core.persist()?;
            println!("Registered hypothesis [{id}]");
        }
        HypothesisCmd::Evidence {
            id,
            claim,
            source,
            polarity,
            weight,
        } => {
            let id = require_hypothesis_id(&core, &id)?;
            let hyp = core
                .hypotheses
                .get_mut(&id)
                .ok_or_else(|| anyhow::anyhow!("hypothesis {id} not found"))?;
            let pol = match polarity.to_lowercase().as_str() {
                "contradicting" | "contra" | "refuting" => EvidencePolarity::Contradicts,
                _ => EvidencePolarity::Supports,
            };
            let ev = Evidence::new(claim, source).with_weight(weight);
            match pol {
                EvidencePolarity::Supports => hyp.add_supporting(ev),
                EvidencePolarity::Contradicts => hyp.add_contradicting(ev),
            }
            core.persist()?;
            println!("Added evidence to hypothesis [{id}]");
        }
        HypothesisCmd::Predict {
            id,
            statement,
            expected,
        } => {
            let id = require_hypothesis_id(&core, &id)?;
            let hyp = core
                .hypotheses
                .get_mut(&id)
                .ok_or_else(|| anyhow::anyhow!("hypothesis {id} not found"))?;
            let mut pred = Prediction::new(statement);
            if let Some(exp) = expected {
                pred.expected_outcome = Some(exp);
            }
            hyp.add_prediction(pred);
            core.persist()?;
            println!("Added prediction to hypothesis [{id}]");
        }
        HypothesisCmd::Transition { id, status, reason } => {
            let id = require_hypothesis_id(&core, &id)?;
            let target_status = match status.to_lowercase().as_str() {
                "candidate" => HypothesisStatus::Candidate,
                "supported" => HypothesisStatus::Supported,
                "contradicted" => HypothesisStatus::Contradicted,
                "confirmed" => HypothesisStatus::Confirmed,
                "inert" => HypothesisStatus::Inert,
                "failed" => HypothesisStatus::Failed,
                "superseded" => HypothesisStatus::Superseded,
                "isolated" => HypothesisStatus::Isolated,
                "certified" => HypothesisStatus::Certified,
                other => anyhow::bail!("unknown status '{other}'"),
            };
            if core.transition_hypothesis(&id, target_status, &reason, None) {
                core.persist()?;
                println!("Hypothesis [{id}] transitioned to {:?}", target_status);
            } else {
                anyhow::bail!("hypothesis {id} not found");
            }
        }
        HypothesisCmd::Explain { id } => {
            let id = require_hypothesis_id(&core, &id)?;
            let report = core
                .full_explanation_report(&id)
                .ok_or_else(|| anyhow::anyhow!("hypothesis {id} not found"))?;
            println!("{}", report.ascii_summary());
        }
        HypothesisCmd::Query {
            topic,
            limit,
            include_dead,
        } => {
            let q = embedder.embed(&topic);
            let mut ranked: Vec<(f32, &Hypothesis)> = core
                .hypotheses
                .values()
                .filter(|h| include_dead || !is_dead(h.status))
                .map(|h| (cosine_sim(&q, &h.embedding), h))
                .collect();
            ranked.sort_by(|a, b| b.0.total_cmp(&a.0));

            if ranked.is_empty() {
                println!("No hypotheses match '{topic}'.");
                println!(
                    "  {} registered in total{}.",
                    core.hypotheses.len(),
                    if include_dead {
                        String::new()
                    } else {
                        format!(
                            ", {} of them dead and hidden (--include-dead to see them)",
                            core.hypotheses
                                .values()
                                .filter(|h| is_dead(h.status))
                                .count()
                        )
                    }
                );
                return Ok(());
            }

            println!("Relevant to '{topic}':");
            for (score, h) in ranked.iter().take(limit) {
                let pending = h.pending_predictions();
                println!(
                    "  [{}] rel={:.3} {:<12} fitness={:.3} ev=+{}/-{} pred={} pending",
                    &h.id[..8],
                    score,
                    format!("{:?}", h.status),
                    h.fitness,
                    h.supporting_evidence.len(),
                    h.contradicting_evidence.len(),
                    pending,
                );
                println!("      {}", h.statement);
            }
            // Relevance alone would launder a speculative claim into an answer.
            // The directive is explicit that an agent which cannot tell
            // certified from speculative will do exactly that, so the split is
            // printed rather than left to the reader.
            let certified = ranked
                .iter()
                .filter(|(_, h)| h.status == HypothesisStatus::Certified)
                .count();
            println!(
                "\n{} shown, {} certified, {} speculative.",
                ranked.len().min(limit),
                certified,
                ranked.len().min(limit) - certified.min(ranked.len().min(limit)),
            );
            // `load_embedder` returns a RandomProjectionEmbedder unconditionally
            // — hashed bag-of-words, no trained weights. Its cosine is high
            // between arbitrary sentences: "stripe payment webhooks in the
            // praxis backend" scores 0.83 against a claim about the ontology
            // delta report. The ORDER is usable; the NUMBER is not, and printing
            // it bare would be describing the system as more capable than the
            // code makes it. Say so on every query rather than in a doc nobody
            // reads at the moment they are trusting the score.
            println!(
                "rel= is random-projection cosine, not a trained embedding. Treat the\n\
                 ordering as a hint and the absolute value as meaningless (unrelated\n\
                 text routinely scores >0.8)."
            );
        }
        HypothesisCmd::Resolve {
            id,
            index,
            actual,
            correct,
            wrong,
        } => {
            if correct == wrong {
                anyhow::bail!(
                    "pass exactly one of --correct or --wrong. \n\
                     Defaulting either way would let a wrong prediction be recorded \n\
                     as right by omission, which is the one thing this command exists \n\
                     to prevent."
                );
            }
            let id = require_hypothesis_id(&core, &id)?;
            let hyp = core
                .hypotheses
                .get_mut(&id)
                .ok_or_else(|| anyhow::anyhow!("hypothesis {id} not found"))?;
            let before = hyp.fitness;
            if !hyp.resolve_prediction(index, &actual, correct) {
                match hyp.predictions.get(index) {
                    Some(p) => anyhow::bail!(
                        "prediction {index} is already resolved (correct={:?}, actual={:?}). \n\
                         Refusing to overwrite it — a record that can be rewritten is not a record.",
                        p.correct,
                        p.actual_outcome
                    ),
                    None => anyhow::bail!(
                        "hypothesis [{}] has {} predictions; {index} is out of range",
                        &id[..8],
                        hyp.predictions.len()
                    ),
                }
            }
            let after = hyp.fitness;
            let status = hyp.status;
            let pending = hyp.pending_predictions();
            core.persist()?;
            println!(
                "Resolved prediction {index} of [{}] as {}.",
                &id[..8],
                if correct { "CORRECT" } else { "WRONG" }
            );
            println!(
                "  fitness {before:.3} -> {after:.3}   status={status:?}   {pending} still pending"
            );
        }
        HypothesisCmd::Open { older_than } => {
            let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than.max(0));
            let mut rows: Vec<(&Hypothesis, usize, &Prediction)> = Vec::new();
            for h in core.hypotheses.values() {
                for (i, p) in h.open_predictions() {
                    if p.made_at <= cutoff {
                        rows.push((h, i, p));
                    }
                }
            }
            rows.sort_by_key(|(_, _, p)| p.made_at);

            if rows.is_empty() {
                println!(
                    "No open predictions{}.",
                    if older_than > 0 {
                        format!(" older than {older_than} days")
                    } else {
                        String::new()
                    }
                );
                return Ok(());
            }
            println!("{} open prediction(s), oldest first:", rows.len());
            for (h, i, p) in rows {
                let age = (chrono::Utc::now() - p.made_at).num_days();
                println!(
                    "  [{}] #{i}  made {age}d ago  (hypothesis: {:?}, fitness {:.3})",
                    &h.id[..8],
                    h.status,
                    h.fitness
                );
                println!("      predicted: {}", p.statement);
                if let Some(e) = &p.expected_outcome {
                    println!("      expected : {e}");
                }
                println!(
                    "      resolve  : physis-core hypothesis resolve {} {i} \"<what happened>\" --correct|--wrong",
                    &h.id[..8]
                );
            }
        }
    }
    Ok(())
}

fn run_contradiction(cmd: ContradictionCmd) -> anyhow::Result<()> {
    let mut core = load_core();
    match cmd {
        ContradictionCmd::List => {
            println!("Recorded contradictions: {}", core.contradictions.len());
            for c in &core.contradictions {
                println!(
                    "  [{}] status={:?} — '{}' ({}) vs '{}' ({})",
                    &c.id[..8],
                    c.resolution,
                    c.claim_a.claim,
                    c.claim_a.source,
                    c.claim_b.claim,
                    c.claim_b.source
                );
            }
        }
        ContradictionCmd::Record {
            claim_a,
            source_a,
            claim_b,
            source_b,
        } => {
            let ca = ContradictionParty::new(claim_a, source_a);
            let cb = ContradictionParty::new(claim_b, source_b);
            let contra = Contradiction::new(ca, cb);
            let id = core.record_contradiction(contra);
            core.persist()?;
            println!("Recorded contradiction [{id}]");
        }
        ContradictionCmd::Resolve {
            id,
            status,
            explanation,
        } => {
            let c = core
                .contradictions
                .iter_mut()
                .find(|c| c.id == id || c.id.starts_with(&id))
                .ok_or_else(|| anyhow::anyhow!("contradiction {id} not found"))?;
            let res = match status.to_lowercase().as_str() {
                "apreferred" | "a" => ResolutionStatus::APreferred,
                "bpreferred" | "b" => ResolutionStatus::BPreferred,
                "contextual" => ResolutionStatus::Contextual,
                "superseded" => ResolutionStatus::Superseded,
                _ => ResolutionStatus::Open,
            };
            c.resolution = res;
            c.explanation = explanation;
            core.persist()?;
            println!("Contradiction [{}] resolved to {:?}", id, res);
        }
    }
    Ok(())
}

fn run_audit() -> anyhow::Result<()> {
    let core = load_core();
    println!(
        "Epistemic audit trail: {} events",
        core.epistemic_audit.events.len()
    );
    println!("{}", core.epistemic_audit.summary());
    Ok(())
}

fn run_replay(subject: &str, at: Option<&str>) -> anyhow::Result<()> {
    // PH-017: project each hypothesis's own revision history into the audit
    // trail before reading it. The trail is a VIEW over `revision_history`
    // rather than a parallel record kept in sync by discipline — discipline is
    // what failed, and the projection removes the disagreement by construction.
    // Idempotent, so the persist below adds nothing when already current.
    let mut core = load_core();
    if core.project_all_revisions() > 0 {
        store::ensure_data_dir()?;
        core.persist()?;
    }
    let timestamp = match at {
        Some(t) => chrono::DateTime::parse_from_rfc3339(t)?.with_timezone(&chrono::Utc),
        None => chrono::Utc::now(),
    };
    // `hypothesis list` prints ids truncated to 8 characters, which is what a
    // reader copies into this flag. Matching only on the full UUID answered
    // "No recorded belief" — indistinguishable from a subject that really has
    // none — so resolve a prefix against the known hypotheses first.
    let subject = match resolve_hypothesis_id(&core, subject) {
        IdMatch::Exact(id) => id,
        IdMatch::Ambiguous(n) => {
            println!("'{subject}' matches {n} hypotheses — use more characters of the id.");
            return Ok(());
        }
        IdMatch::Unknown => subject.to_string(),
    };

    let replayed = core.reconstruct_belief_at(&subject, timestamp);
    let derived = core.hypotheses.get(&subject).map(|h| h.status);

    match replayed {
        Some(status) => println!("Belief at {timestamp} for [{subject}]: {:?}", status),
        None if core.hypotheses.contains_key(&subject) => {
            println!("[{subject}] is known but had no recorded belief at {timestamp}")
        }
        None => println!("No subject [{subject}] in the audit trail (try `physis-core audit`)"),
    }

    // PH-017: where the log and the derived view disagree, SAY SO.
    //
    // Answering with one of them silently is the original defect wearing a
    // different face. The log is authoritative for what was RECORDED; the
    // derived status is what the evidence implies now. A gap between them is
    // information — usually that the claim's history was written before the
    // `revise` fix, when every recorded transition had both ends set to the
    // destination and so said nothing moved. That history cannot be
    // reconstructed and must not be fabricated.
    if at.is_none() {
        if let (Some(d), Some(r)) = (derived, replayed) {
            if d != r {
                println!();
                println!("  !! the log and the derived view DISAGREE");
                println!("     recorded (this replay) : {r:?}");
                println!("     derived from evidence  : {d:?}");
                println!("     The log has no transition for this claim — its revisions predate");
                println!("     the PH-017 fix and recorded no change. The gap is not repairable");
                println!("     without inventing history, so it is reported instead.");
            }
        }
    }
    Ok(())
}

/// Outcome of matching a possibly-abbreviated id against the known hypotheses.
enum IdMatch {
    Exact(String),
    Ambiguous(usize),
    Unknown,
}

/// Resolve a full id, or an unambiguous prefix of one, to a hypothesis id.
fn resolve_hypothesis_id(core: &PhysisCore, needle: &str) -> IdMatch {
    if core.hypotheses.contains_key(needle) {
        return IdMatch::Exact(needle.to_string());
    }
    let mut hits = core.hypotheses.keys().filter(|id| id.starts_with(needle));
    match (hits.next(), hits.next()) {
        (Some(id), None) => IdMatch::Exact(id.clone()),
        (Some(_), Some(_)) => IdMatch::Ambiguous(
            core.hypotheses
                .keys()
                .filter(|id| id.starts_with(needle))
                .count(),
        ),
        _ => IdMatch::Unknown,
    }
}

fn run_discover(dir: Option<&std::path::Path>, min_cluster: usize) -> anyhow::Result<()> {
    let (embedder, kind) = load_embedder_kind();
    let ontology = OntologyLoader::load_all();
    let classifier = build_classifier(&ontology, embedder.as_ref(), kind);
    let mut texts = Vec::new();

    if let Some(d) = dir {
        for entry in walk(d)? {
            if let Ok(text) = std::fs::read_to_string(&entry) {
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.len() > 10 {
                        texts.push(trimmed.to_string());
                    }
                }
            }
        }
    } else {
        let core = load_core();
        for node in core.nodes.values() {
            if let Some(ref l) = node.label {
                texts.push(l.clone());
            }
        }
    }

    if texts.is_empty() {
        println!("No texts available for ontology gap discovery.");
        return Ok(());
    }

    let config = physis_core::discovery::DiscoveryConfig {
        min_cluster,
        ..Default::default()
    };
    let report = physis_core::discovery::discover(&texts, &classifier, &embedder, &config);

    println!(
        "Ontology Gap Discovery: evaluated {} texts ({} unmapped)",
        report.total, report.unmapped
    );
    for prop in &report.proposals {
        println!(
            "\n  Proposed Domain: {} ({} × {}) — {} texts",
            prop.name,
            prop.domain,
            prop.mode,
            prop.samples.len()
        );
        println!("    Hints: {}", prop.hints.join(", "));
        const SHOWN: usize = 2;
        for s in prop.samples.iter().take(SHOWN) {
            // Samples are free text and may be multi-line; one line each keeps
            // the list readable and the indentation intact. Collapse rather than
            // take the first line, so a sample whose head is a markdown title
            // still shows the body that put it in this cluster.
            println!("    - {}", elide_middle(&single_line(s), 100));
        }
        // The header advertises the cluster size, so a silently truncated list
        // reads as a miscount rather than as an elision.
        if let Some(rest) = prop.samples.len().checked_sub(SHOWN).filter(|n| *n > 0) {
            println!("    … and {rest} more");
        }
    }
    Ok(())
}

#[cfg(feature = "studio")]
#[derive(Subcommand)]
enum ModelCmd {
    /// List installed models.
    List,
    /// Show the manifest of one installed model.
    Info { id: String },
    /// Install a model from a local directory (checksum-verified).
    Install {
        id: String,
        /// Source directory containing the model files.
        #[arg(long)]
        src: PathBuf,
        /// Files to install (relative to --src).
        #[arg(long = "file", required = true)]
        files: Vec<String>,
    },
    /// Load a registry model or table through the model interface and print
    /// its real backend card (what the engine will actually run).
    Use { id: String },
    /// Remove an installed model.
    Remove { id: String },
    /// Print the install path of a model id.
    Path { id: String },
}

#[derive(Subcommand)]
enum NGramCmd {
    /// Build a table from a corpus directory.
    Build {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 5)]
        order: u8,
        #[arg(long, default_value_t = 1)]
        min_count: usize,
        #[arg(long, default_value_t = 50000)]
        max_vocabulary: usize,
        /// Build a structural (grid-symbol) table instead of a lexical one.
        #[arg(long)]
        structural: bool,
    },
    List,
    Info {
        id: String,
    },
    Remove {
        id: String,
    },
    Path {
        id: String,
    },
    /// Human-readable continuation inspection of an installed table.
    Inspect {
        id: String,
        #[arg(long)]
        context: String,
        #[arg(long, default_value_t = 8)]
        top: usize,
    },
    /// Build a per-cell table family over a corpus and report each cell's
    /// support against the flat control built from the same sequences.
    ///
    /// The flat table is not a formality: conditioning costs data, so whether
    /// per-cell specificity pays for the sparsity is empirical. This prints
    /// both so the comparison is in front of whoever runs it.
    Cells {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value_t = 3)]
        order: u8,
        #[arg(long, default_value_t = 1)]
        min_count: usize,
        /// Cells below this support fall back to the flat control.
        #[arg(long, default_value_t = 10)]
        min_support: usize,
    },
    Export {
        id: String,
        dest: PathBuf,
    },
    Import {
        id: String,
        src: PathBuf,
    },
}

fn cmd_model(cmd: ModelCmd) -> anyhow::Result<()> {
    use physis_core::model_provider::ModelProvider;
    use physis_core::ngram_table::NGramTable;
    let reg = physis_core::model_provider::ModelRegistry::new(
        physis_core::model_provider::ModelRegistry::default_root(),
    );
    match cmd {
        ModelCmd::List => {
            for m in reg.list() {
                println!(
                    "{:<24} {:<28} {} MB",
                    m.record.id, m.record.architecture, m.record.memory_estimate_mb
                );
            }
            if reg.list().is_empty() {
                eprintln!("no models installed — install one with `model install --id … --src …`");
            }
        }
        ModelCmd::Info { id } => {
            let m = reg.info(&id)?;
            println!("{}", serde_json::to_string_pretty(&m)?);
        }
        ModelCmd::Install { id, src, files } => {
            let record = physis_core::model_provider::ModelRecord {
                id: id.clone(),
                name: id.clone(),
                architecture: "local".into(),
                parameter_count: None,
                context_length: 0,
                tokenizer: "whitespace-v1".into(),
                quantization: None,
                source: format!("local:{}", src.display()),
                memory_estimate_mb: 0,
                capabilities: vec![physis_core::model_provider::Capability::Generation],
                license: "unspecified".into(),
            };
            let m = reg.install_local(record, &src, &files)?;
            println!(
                "installed {} ({} files verified)",
                m.record.id,
                m.files.len()
            );
        }
        ModelCmd::Use { id } => {
            let mreg = physis_core::model_provider::ModelRegistry::new(
                physis_core::model_provider::ModelRegistry::default_root(),
            );
            let nreg = physis_core::ngram_table::TableRegistry::new(
                physis_core::ngram_table::TableRegistry::default_root(),
            );
            let mdir = mreg.path(&id);
            let tdir = nreg.path(&id);
            if mdir.is_dir() {
                let m = physis_core::model_provider::DiskTableModel::load(&id, &mdir)?;
                println!("{}", serde_json::to_string_pretty(&m.metadata())?);
            } else if tdir.parent().map(|d| d.is_dir()).unwrap_or(false) {
                let t = nreg.load(&id, None)?;
                let kind = match t.manifest().kind {
                    physis_core::ngram_table::TableKind::Lexical => "lexical".to_string(),
                    physis_core::ngram_table::TableKind::Structural => "structural".to_string(),
                };
                let model = physis_core::model_provider::NgramDecoderModel::new(
                    &id,
                    std::sync::Arc::new(t),
                    Box::new(physis_core::tokenizer::WhitespaceTokenizer::new(50_000)),
                );
                let info = serde_json::json!({
                    "id": model.metadata().id,
                    "architecture": model.metadata().architecture,
                    "parameters": model.metadata().parameter_count,
                    "tokenizer": model.metadata().tokenizer,
                    "capabilities": model.metadata().capabilities,
                    "kind": kind,
                });
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                return Err(anyhow::anyhow!(
                    "no installed model or table under id '{}' — install it first with model install / ngram import",
                    id
                ));
            }
        }
        ModelCmd::Remove { id } => {
            reg.remove(&id)?;
            println!("removed {id}");
        }
        ModelCmd::Path { id } => println!("{}", reg.path(&id).display()),
    }
    Ok(())
}

fn cmd_ngram(cmd: NGramCmd) -> anyhow::Result<()> {
    use physis_core::ngram_table::{self, NGramTable, TableConfig, TableKind, TableRegistry};
    use physis_core::tokenizer::{Tokenizer, WhitespaceTokenizer};
    let reg = TableRegistry::new(TableRegistry::default_root());
    match cmd {
        NGramCmd::Build {
            input,
            output,
            order,
            min_count,
            max_vocabulary,
            structural,
        } => {
            let docs = physis_core::map::load_corpus(&input)?;
            anyhow::ensure!(
                !docs.is_empty(),
                "no md/txt documents under {}",
                input.display()
            );
            let cfg = TableConfig {
                table_id: output
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "table".into()),
                kind: if structural {
                    TableKind::Structural
                } else {
                    TableKind::Lexical
                },
                max_order: order,
                min_count,
                max_vocabulary,
                ..Default::default()
            };
            let (table, bytes) = if structural {
                let ontology = physis_core::ontology::OntologyLoader::load_all();
                // A structural table IS the classifier's output, one symbol per
                // segment. Built on a lexical hash the symbol sequence is noise
                // with a schema, and every downstream order-5 count estimates a
                // distribution over misfilings. This was the last site still
                // constructing its own RandomProjectionEmbedder directly.
                let (embedder, kind) = load_embedder_kind();
                let clf = build_classifier(&ontology, embedder.as_ref(), kind);
                ngram_table::build_structural(&docs, cfg, |seg| {
                    let emb = embedder.as_ref().embed(seg);
                    clf.best_entry_sim(&emb)
                        .map(|(_, d, m)| format!("{d}×{m}"))
                        .unwrap_or_else(|| "UNKNOWN×UNKNOWN".into())
                })?
            } else {
                let t = WhitespaceTokenizer::new(max_vocabulary);
                let mut b = ngram_table::TableBuilder::new(cfg);
                for (_, body) in &docs {
                    b.push_text(&t, body);
                }
                let mut h = Sha256::new();
                for (pth, bd) in &docs {
                    h.update(pth.as_bytes());
                    h.update(bd.as_bytes());
                }
                let hash = format!("{:x}", h.finalize());
                b.finish(&t, &hash)
            };
            std::fs::create_dir_all(output.parent().unwrap_or(Path::new(".")))?;
            std::fs::write(&output, &bytes)?;
            let st = table.statistics();
            println!(
                "built {} — {} entries, vocab {}, max order {}, {} bytes, sha-checksummed",
                table.manifest().table_id,
                st.entry_count,
                st.vocabulary,
                st.max_order,
                bytes.len()
            );
        }
        NGramCmd::List => {
            let all = reg.list();
            if all.is_empty() {
                eprintln!("no tables — build one with `ngram build --input … --output …`");
            }
            for (id, m) in all {
                println!(
                    "{:<28} {:?} order {} {} entries",
                    id, m.kind, m.max_order, m.entry_count
                );
            }
        }
        NGramCmd::Info { id } => println!("{}", serde_json::to_string_pretty(&reg.info(&id)?)?),
        NGramCmd::Remove { id } => {
            reg.remove(&id)?;
            println!("removed {id}");
        }
        NGramCmd::Path { id } => println!("{}", reg.path(&id).display()),
        NGramCmd::Inspect { id, context, top } => {
            let table = reg.load(&id, None)?;
            let t = WhitespaceTokenizer::new(0);
            let ctx = t.encode(&context);
            println!(
                "CONTEXT
{context}
"
            );
            let m = table.manifest();
            println!("BACKOFF ORDER {} → {}", m.max_order, 1);
            println!(
                "
CANDIDATES (count-ordered)
"
            );
            println!("{:<24} probability", "token");
            println!("-----------------------------------");
            for (w, p) in table.top_next(&ctx, top) {
                println!("{:<24} {:.4}", w, p);
            }
        }
        NGramCmd::Cells {
            input,
            order,
            min_count,
            min_support,
        } => {
            return cmd_ngram_cells(&input, order, min_count, min_support);
        }
        NGramCmd::Export { id, dest } => {
            let n = reg.export(&id, &dest)?;
            println!("exported {id} → {} ({n} bytes)", dest.display());
        }
        NGramCmd::Import { id, src } => {
            let m = reg.import(&id, &src)?;
            println!(
                "imported {} (order {}, {} entries)",
                id, m.max_order, m.entry_count
            );
        }
    }
    Ok(())
}

fn cmd_demo(dir: &Path, query: &str, order: u8) -> anyhow::Result<()> {
    use physis_core::model_provider::{ModelProvider, NgramDecoderModel};
    use physis_core::ngram_table::{TableBuilder, TableConfig};
    use std::sync::Arc;
    let docs = physis_core::map::load_corpus(dir)?;
    anyhow::ensure!(
        !docs.is_empty(),
        "no corpus documents under {}",
        dir.display()
    );
    let embedder = load_embedder();
    let report = physis_core::map::build_map(&docs, &embedder, None)?;
    println!("── PHYSIS STRUCTURAL MAP (deterministic) ──");
    for l in report.hero_lines() {
        println!("{l}");
    }
    // The n-gram floor + model, built in-process from the same corpus.
    let tok = physis_core::tokenizer::WhitespaceTokenizer::new(0);
    let cfg = TableConfig {
        table_id: "demo".into(),
        max_order: order,
        ..Default::default()
    };
    let mut b = TableBuilder::new(cfg);
    for (_, body) in &docs {
        b.push_text(&tok, body);
    }
    let (table, _bytes) = b.finish(&tok, "demo-corpus");
    let model = NgramDecoderModel::new("demo-ngram", Arc::new(table), Box::new(tok));
    println!(
        "
── LOCAL MODEL (n-gram decoder, offline, deterministic) ──"
    );
    println!(
        "model: {} — caps {:?} — ~{} MB",
        model.metadata().id,
        model.capabilities(),
        model.memory_requirements_mb()
    );
    println!("score(query) = {:.4} mean log-prob", model.score(query)?);
    println!("continuation: {}…", model.generate(query, 8)?);
    println!(
        "
less context, more structure — physis."
    );
    Ok(())
}

fn cmd_watch(source: &str, path: &Path, max: usize, dry_run: bool) -> anyhow::Result<()> {
    let log = physis_core::observe::log_path();
    // Bounded tail, not the whole log. Deduplication only needs recent history,
    // and reading everything made a `watch` run cost 2.0 s / 372 MB at 500k
    // observations — the cliff that killed Nepomuk, measured arriving here.
    // 20k is far more than any watcher's window (max is capped at 500).
    let known = physis_core::observe::read_tail(&log, 20_000)?;
    let home = std::env::var("HOME").unwrap_or_default();
    let shell_hist = || {
        let z = PathBuf::from(&home).join(".zsh_history");
        if z.exists() {
            z
        } else {
            PathBuf::from(&home).join(".bash_history")
        }
    };
    let agent_dir = || PathBuf::from(&home).join(".claude/projects");
    // First browser profile that exists. Firefox and Chromium have different
    // schemas and different epochs; `watch_browser` picks by filename.
    let browser_db = || -> Option<PathBuf> {
        let chromium = PathBuf::from(&home).join(".config/chromium/Default/History");
        if chromium.exists() {
            return Some(chromium);
        }
        let chrome = PathBuf::from(&home).join(".config/google-chrome/Default/History");
        if chrome.exists() {
            return Some(chrome);
        }
        std::fs::read_dir(PathBuf::from(&home).join(".mozilla/firefox"))
            .ok()?
            .flatten()
            .map(|e| e.path().join("places.sqlite"))
            .find(|p| p.exists() && !p.to_string_lossy().contains(".bak-"))
    };

    let mut fresh = match source {
        "fs" => physis_core::observe::watch_fs(path, &known, max),
        "git" => physis_core::observe::watch_git(path, max.min(500)),
        "terminal" => physis_core::observe::watch_shell(&shell_hist(), &known, max.min(500)),
        "agent" => physis_core::observe::watch_agent(&agent_dir(), &known, max.min(500)),
        "process" => physis_core::observe::watch_proc(&known, 60, max.min(500)),
        "browser" => browser_db()
            .map(|db| physis_core::observe::watch_browser(&db, &known, max.min(500)))
            .unwrap_or_default(),
        // The point of the substrate is that these are one timeline, so running
        // them together is the default way to use it rather than a convenience.
        "all" => {
            let mut v = physis_core::observe::watch_fs(path, &known, max);
            v.extend(physis_core::observe::watch_git(path, max.min(200)));
            v.extend(physis_core::observe::watch_shell(
                &shell_hist(),
                &known,
                max.min(200),
            ));
            v.extend(physis_core::observe::watch_agent(
                &agent_dir(),
                &known,
                max.min(200),
            ));
            v.extend(physis_core::observe::watch_proc(&known, 60, max.min(200)));
            if let Some(db) = browser_db() {
                v.extend(physis_core::observe::watch_browser(
                    &db,
                    &known,
                    max.min(200),
                ));
            }
            // One timeline: sort by when it happened, not by which watcher ran.
            v.sort_by_key(|o| o.at);
            v
        }
        other => anyhow::bail!(
            "unknown source {other:?} — try fs | git | terminal | agent | process | browser | all"
        ),
    };
    // `git` re-reads the same history every run; drop what the log already has
    // so the append stays idempotent the way the other watchers already are.
    fresh.retain(|o| {
        !known
            .iter()
            .any(|k| k.source == o.source && k.subject == o.subject)
    });
    if fresh.is_empty() {
        println!(
            "nothing changed — {} observation(s) already in the log",
            known.len()
        );
        return Ok(());
    }
    if dry_run {
        println!("would append {} observation(s):", fresh.len());
        for o in fresh.iter().take(20) {
            println!(
                "  {:<9} {}",
                o.source,
                o.subject.chars().take(80).collect::<String>()
            );
        }
        return Ok(());
    }
    let mut by: std::collections::BTreeMap<String, usize> = Default::default();
    for o in fresh.iter() {
        *by.entry(o.source.clone()).or_default() += 1;
    }
    let (a, b) = physis_core::observe::append(&log, &mut fresh)?;
    let breakdown: Vec<String> = by.iter().map(|(k, n)| format!("{n} {k}")).collect();
    println!(
        "appended {} observation(s), seq {a}..={b}  ({})",
        fresh.len(),
        breakdown.join(" · ")
    );
    println!("log: {}", log.display());
    Ok(())
}

fn cmd_observed(
    source: Option<&str>,
    concurrent: Option<u64>,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    let all = physis_core::observe::read(&physis_core::observe::log_path())?;
    if all.is_empty() {
        println!("The log is empty. Nothing has been observed, so nothing can be claimed.");
        println!("  physis-core watch --source fs --path .");
        return Ok(());
    }
    let rows: Vec<physis_core::observe::Observation> = if let Some(seq) = concurrent {
        physis_core::observe::concurrent(&all, seq)
    } else if let Some(s) = source {
        physis_core::observe::by_source(&all, s)
    } else {
        let mut v = all.clone();
        v.sort_by_key(|o| std::cmp::Reverse(o.seq));
        v
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }
    if let Some(seq) = concurrent {
        println!("── CONCURRENT WITH #{seq} ──");
        if rows.is_empty() {
            println!("  nothing overlapped it in time");
        }
    } else {
        println!("── OBSERVED ── {} total", all.len());
    }
    for o in rows.iter().take(limit) {
        let dur = o
            .duration_ms
            .map(|m| format!("{:>7.1}s", m as f64 / 1000.0))
            .unwrap_or_else(|| "       ·".into());
        println!(
            "  #{:<5} {} {:<9} {}",
            o.seq,
            dur,
            o.source,
            o.subject.chars().take(84).collect::<String>()
        );
    }
    Ok(())
}

fn cmd_act(command: &str, dry_run: bool, top: usize, json: bool) -> anyhow::Result<()> {
    let (embedder, _) = physis_core::embed::select(384);
    let core = load_core();
    let bearing = physis_core::act::bearing_on(&core, command, embedder.as_ref(), top);

    if dry_run {
        let warnings: Vec<_> = bearing.iter().filter(|b| b.is_warning()).collect();
        if warnings.is_empty() {
            println!(
                "nothing already established bears on this. {} claim(s) checked.",
                core.hypotheses.len()
            );
        } else {
            println!("── ALREADY ESTABLISHED ──");
            for b in warnings {
                println!(
                    "  [{}] {:<13} rel {:.3}\n      {}",
                    b.id, b.status, b.relevance, b.statement
                );
            }
        }
        println!("\n(not run — drop --dry-run to execute)");
        return Ok(());
    }

    store::ensure_data_dir()?;
    let a = physis_core::act::run(command, bearing, &physis_core::observe::log_path())?;
    if json {
        println!("{}", serde_json::to_string_pretty(&a)?);
    } else {
        print!("{}", a.render());
    }
    // The command's own exit code is the caller's result; a substrate that
    // swallowed it would break every script that wraps this.
    if a.exit_code != Some(0) {
        std::process::exit(a.exit_code.unwrap_or(1));
    }
    Ok(())
}

fn cmd_claim(from: u64, statement: &str, confidence: f32) -> anyhow::Result<()> {
    let log = physis_core::observe::log_path();
    let all = physis_core::observe::read(&log)?;
    let Some(obs) = all.iter().find(|o| o.seq == from) else {
        anyhow::bail!(
            "no observation #{from} in the log ({} recorded). `physis-core observed` lists them.",
            all.len()
        );
    };
    let (embedder, _) = physis_core::embed::select(384);
    let mut h = physis_core::observe::promote(obs, statement, embedder.embed(statement));
    h.confidence = confidence;

    let mut core = load_core();
    let id = h.id.clone();
    core.hypotheses.insert(id.clone(), h);
    store::ensure_data_dir()?;
    core.persist()?;

    println!("claim [{}] registered", &id[..8]);
    println!("  cites obs:{} — {} {}", obs.seq, obs.source, obs.subject);
    println!();
    println!("It is a Candidate: asserted, not established. Refute or support it:");
    println!(
        "  physis-core hypothesis evidence {} \"<measurement>\" --polarity contradicting",
        &id[..8]
    );
    println!("  physis-core ground        # see it beside everything else believed");
    Ok(())
}

fn cmd_direction(before: &Path, after: &Path, want: &str, json: bool) -> anyhow::Result<()> {
    let Some(dir) = physis_core::direction::Direction::parse(want) else {
        anyhow::bail!("unknown direction {want:?} — try shorter | longer | rephrased");
    };
    let b = std::fs::read_to_string(before)?;
    let a = std::fs::read_to_string(after)?;
    let (embedder, kind) = physis_core::embed::select(384);
    let v = physis_core::direction::check(&b, &a, dir, embedder.as_ref(), kind)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        print!("{}", v.render());
    }
    // A failed direction check must be visible to a script, not only to a reader.
    if !v.holds() {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_chain(
    corpus: &Path,
    query: &str,
    budget: usize,
    threshold: f32,
    json: bool,
) -> anyhow::Result<()> {
    let docs = physis_core::map::load_corpus(corpus)?;
    anyhow::ensure!(
        !docs.is_empty(),
        "no corpus documents under {}",
        corpus.display()
    );
    let (embedder, kind) = load_embedder_kind();
    let ontology = physis_core::ontology::OntologyLoader::load_all();
    let clf = build_classifier(&ontology, embedder.as_ref(), kind);
    // An empty query still needs something to compile context against; the
    // corpus's own first title is a neutral default that keeps the run
    // reproducible instead of erroring on a flag the user did not need.
    let q = if query.trim().is_empty() {
        docs[0].0.as_str()
    } else {
        query
    };
    let r = physis_core::chain::run(&docs, q, embedder.as_ref(), &clf, budget, threshold, kind)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&r)?);
    } else {
        print!("{}", r.render());
    }
    // Exit non-zero when there WAS a comparison and the pass lost it, so a
    // script cannot consume a shortlist the chain itself calls noise. A corpus
    // with no gaps is not a failure: nothing was proposed, so nothing is unsafe
    // to act on, and exiting 1 there would make "clean corpus" look like "broken
    // run".
    if r.testable() && !r.discriminates() {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_notebook(
    corpus: &Path,
    query: &str,
    budget: usize,
    json: bool,
    draft: bool,
    confidence: f32,
) -> anyhow::Result<()> {
    let docs = physis_core::map::load_corpus(corpus)?;
    anyhow::ensure!(
        !docs.is_empty(),
        "no corpus documents under {}",
        corpus.display()
    );
    let embedder = load_embedder();

    if draft {
        let texts: Vec<String> = docs.iter().map(|(_, b)| b.clone()).collect();
        let top_k = docs.len().clamp(3, 8);
        let (result, _) = physis_core::rag::retrieve_from_texts(
            &texts,
            query,
            embedder.as_ref(),
            budget.max(64),
            top_k,
        );
        let d = physis_core::notebook::draft(&result, query, 40, confidence)?;
        if json {
            println!("{}", serde_json::to_string_pretty(&d)?);
            return Ok(());
        }
        println!("── DRAFT (table over the retrieved context) ──\n");
        println!("{}\n", d.render_with_gaps().trim());
        println!(
            "table supplied {} token(s), {} gap(s) left for a model",
            d.drafted_tokens, d.gaps
        );
        println!(
            "table_share    {:.2}   (table entries: {})",
            d.table_share(),
            d.table_entries
        );
        // The number decides whether the architecture applies; say so rather
        // than leaving the reader to infer it from a bare float.
        if d.table_share() >= 0.5 {
            println!("\nThe corpus carries most of this answer: a small model only has");
            println!("to close {} gap(s).", d.gaps);
        } else {
            println!("\nThe corpus carries little of this answer. Draft-and-fill adds");
            println!("latency here for no saving — ask a model directly.");
        }
        return Ok(());
    }

    let a = physis_core::notebook::answer(&docs, query, embedder.as_ref(), budget)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&a)?);
    } else {
        print!("{}", a.render());
    }
    Ok(())
}

fn cmd_ngram_cells(
    input: &Path,
    order: u8,
    min_count: usize,
    min_support: usize,
) -> anyhow::Result<()> {
    use physis_core::ngram_table::{build_cell_tables, TableConfig, TableKind};
    let docs = physis_core::map::load_corpus(input)?;
    anyhow::ensure!(
        !docs.is_empty(),
        "no corpus documents under {}",
        input.display()
    );
    let (embedder, kind) = load_embedder_kind();
    let ontology = physis_core::ontology::OntologyLoader::load_all();
    let clf = build_classifier(&ontology, embedder.as_ref(), kind);

    // One ordered symbol run per document: these are transitions, and a set
    // would lose exactly the information the tables are for.
    let sequences: Vec<Vec<String>> = docs
        .iter()
        .map(|(_, body)| {
            body.split(['\n', '.'])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|seg| {
                    clf.best_entry_sim(&embedder.embed(seg))
                        .map(|(_, d, m)| format!("{d}×{m}"))
                        .unwrap_or_else(|| "UNKNOWN×UNKNOWN".into())
                })
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .collect();

    let cfg = TableConfig {
        kind: TableKind::Structural,
        max_order: order,
        min_count,
        ..Default::default()
    };
    let fam = build_cell_tables(&sequences, cfg)?;

    println!("── PER-CELL TABLE FAMILY ──");
    // Two different counts, previously conflated into one confusing line
    // ("0 of 2" printed above a list of 9 cells): `support` counts every cell
    // OCCURRENCE, while a cell only gets a table if it was ever followed by
    // something. A cell appearing once, at the end of a document, has support
    // but no transitions and therefore no table.
    println!(
        "{} document(s) → {} sequence(s)",
        docs.len(),
        sequences.len()
    );
    println!(
        "{} cell(s) seen, {} with a following transition (so with a table)",
        fam.support.len(),
        fam.tables.len()
    );
    println!("min_support {min_support}: cells below it fall back to the flat control\n");
    println!("{:<28} {:>8}  OWN TABLE?", "CELL", "SUPPORT");
    let mut used = 0;
    for (cell, n) in fam.by_support() {
        let own = n >= min_support && fam.tables.contains_key(cell);
        used += own as usize;
        println!(
            "{:<28} {:>8}  {}",
            cell,
            n,
            if own { "yes" } else { "flat" }
        );
    }
    println!(
        "\n{used} of {} cell(s) with a table carry support >= {min_support}.",
        fam.tables.len()
    );
    println!("The flat control was built from the same sequences in the same pass.");
    println!("Compare against it before claiming per-cell tables help.");
    Ok(())
}

fn cmd_context(corpus: &Path, query: &str, budget: usize, json: bool) -> anyhow::Result<()> {
    use physis_core::map::compile_context;
    let docs = physis_core::map::load_corpus(corpus)?;
    anyhow::ensure!(
        !docs.is_empty(),
        "no corpus documents under {}",
        corpus.display()
    );
    let embedder = load_embedder();
    let report = compile_context(&docs, &embedder, query, budget)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", report.render());
        println!(
            "
── COMPILED CONTEXT (first {} chars) ──",
            400
        );
        println!(
            "{}",
            report.physis_context.chars().take(400).collect::<String>()
        );
    }
    Ok(())
}

fn cmd_benchmark(order: u8, budget: usize, big_model: Option<String>) -> anyhow::Result<()> {
    use physis_core::bench::BenchConfig;
    // A benchmark that does not record WHICH embedder produced it is not
    // reproducible: the same corpus scores differently under a lexical hash and
    // a real model. `embedder_kind` goes into provenance.json for that reason.
    let (embedder, embedder_kind) = physis_core::embed::select(384);
    if embedder_kind == "random-projection" {
        eprintln!(
            "warning: benchmarking with random-projection (non-semantic). \
Retrieval quality here is lexical; the numbers are a floor, not a claim."
        );
    }
    let pkg = env!("CARGO_PKG_VERSION").to_string();
    let cfg = BenchConfig {
        corpus_root: "benchmarks/ground-truth".into(),
        order,
        min_count: 1,
        context_budget: budget,
        big_model,
        physis_version: pkg.clone(),
        git_commit: physis_core::bench::git_head().clone(),
        seed: 7,
    };
    let out = physis_core::bench::run(cfg.clone(), embedder.as_ref())?;
    let adir = out.artifacts_dir.clone();
    physis_core::bench::materialise_ground_truth(&PathBuf::from(cfg.corpus_root))?;
    std::fs::create_dir_all(&adir)?;
    std::fs::write(
        PathBuf::from(&adir).join("run.json"),
        serde_json::to_vec_pretty(&out.run)?,
    )?;
    std::fs::write(
        PathBuf::from(&adir).join("metrics.json"),
        serde_json::to_vec_pretty(&out.run.metrics)?,
    )?;
    std::fs::write(
        PathBuf::from(&adir).join("provenance.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "corpus_hash": out.run.corpus_hash,
            "git_commit": cfg.git_commit,
            "physis_version": cfg.physis_version,
            "seed": cfg.seed,
            "model": cfg.big_model,
            "embedder": embedder_kind,
            "ngram_order": cfg.order,
            "context_budget": cfg.context_budget,
        }))?,
    )?;
    let m = out.run.metrics;
    let rp = (m.repeat_recovery * 100.0).round();
    let ap = (m.anomaly_recall * 100.0).round();
    let cp = (m.contradiction_recall * 100.0).round();
    let cpct = (m.context_compression_ratio * 100.0).round();
    println!("BENCHMARK (ground truth, deterministic)");
    println!(
        "repeat/recall        {}% ({}/{})",
        rp, m.families_found, m.families_known
    );
    println!(
        "anomaly recall       {}% ({}/{})",
        ap, m.anomalies_caught, m.anomalies_known
    );
    println!(
        "contradiction recall {}% ({}/{})",
        cp, m.contradictions_found, m.contradictions_known
    );
    println!(
        "map deterministic    {} ({})",
        m.map_deterministic,
        m.structure_hash.chars().take(12).collect::<String>()
    );
    println!(
        "context compression  {}% ({} to {} tokens)",
        cpct, m.baseline_tokens, m.physis_tokens
    );
    println!(
        "ngram build {} ms, load {} ms, {} lookups/s, {} bytes on disk",
        m.ngram_build_ms.round(),
        m.ngram_load_ms.round(),
        m.ngram_lookup_per_sec.round(),
        m.ngram_disk_bytes
    );
    println!("ngram RAM estimate   {} MB", m.ngram_ram_estimate_mb);
    println!("embedder             {}", embedder_kind);
    println!("interchangeable      {}", m.interchange_ok);
    println!(
        "held-out ({} docs) score  {}",
        m.heldout_docs,
        (m.heldout_score * 100.0).round()
    );
    println!("big-model oracle leg {}", m.big_model_leg);
    println!(
        "oracle agreement     {}% · evidence {}/{} · per-case detail in metrics.json oracle_cases",
        (m.oracle_mean_agreement * 100.0).round(),
        m.oracle_evidence_hits,
        m.oracle_evidence_total
    );
    println!("artifacts -> benchmarks/results/ (run.json, metrics.json, provenance.json)");
    Ok(())
}

fn cmd_run(
    config: &Path,
    model: Option<String>,
    ngram: Option<String>,
    query: Option<String>,
) -> anyhow::Result<()> {
    use physis_core::model_provider::{ModelProvider, NgramDecoderModel};
    use physis_core::ngram_table::{
        NGramTable, TableBuilder, TableConfig, TableKind, TableRegistry,
    };
    use physis_core::tokenizer::WhitespaceTokenizer;
    use std::sync::Arc;
    let mut cfg = physis_core::config_run::load_config(&PathBuf::from(config))?;
    if let Some(q) = query {
        cfg.query = q.to_string();
    }
    if let Some(id) = ngram {
        cfg.ngram.registry_id = Some(id.to_string());
    }
    // Resolve the ngram: registry id if given, else build from the corpus.
    let table = if let Some(rid) = cfg.ngram.registry_id {
        let reg = TableRegistry::new(TableRegistry::default_root());
        Arc::new(reg.load(&rid, None)?)
    } else {
        let docs = physis_core::map::load_corpus(&cfg.corpus)?;
        let tok = WhitespaceTokenizer::new(50_000);
        let bcfg = TableConfig {
            table_id: "run".into(),
            max_order: cfg.ngram.order,
            ..Default::default()
        };
        let mut tb = TableBuilder::new(bcfg);
        for (_, b) in &docs {
            tb.push_text(&tok, b);
        }
        let (t, _bytes) = tb.finish(&tok, "run-corpus");
        Arc::new(t)
    };
    let kind = match table.manifest().kind {
        TableKind::Lexical => "lexical".to_string(),
        TableKind::Structural => "structural".to_string(),
    };
    let model_id = if let Some(mx) = model {
        mx.to_string()
    } else if let Some(cm) = cfg.model {
        cm
    } else {
        "demo-ngram".to_string()
    };
    let m = NgramDecoderModel::new(&model_id, table, Box::new(WhitespaceTokenizer::new(50_000)));
    let docs = physis_core::map::load_corpus(&cfg.corpus)?;
    let embedder = load_embedder();
    let map = physis_core::map::build_map(&docs, embedder.as_ref(), None)?;
    let ctx = physis_core::map::compile_context(&docs, &embedder, &cfg.query, cfg.budget)?;
    let score = match m.score(&cfg.query) {
        Ok(x) => (x * 100.0).round(),
        Err(_) => 0.0,
    };
    let cont = m.generate(&cfg.query, 8)?;
    println!("── PHYSIS RUN (config-driven, offline, deterministic) ──");
    println!(
        "model: {} · architecture: {} · ngram kind: {}",
        m.metadata().id,
        m.metadata().architecture,
        kind
    );
    println!("corpus: {} · query: {}", cfg.corpus.display(), cfg.query);
    for l in map.hero_lines() {
        println!("{l}");
    }
    println!(
        "context compression {}% ({} to {} tokens)",
        (ctx.compression_ratio * 100.0).round(),
        ctx.baseline_tokens,
        ctx.physis_tokens
    );
    println!("score(query) {} (mean log-prob * 100)", score);
    println!(
        "continuation: {} ...",
        cont.trim().chars().take(60).collect::<String>()
    );
    println!("less context, more structure — physis.");
    Ok(())
}

fn run_studio(port: u16, model: Option<String>) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(physis_core::studio::run_with_model(port, model))
}

/// Load the persisted core (or a fresh one).
fn load_core() -> PhysisCore {
    let json = store::read_optional(&store::nodes_path());
    if json.trim().is_empty() {
        PhysisCore::new()
    } else {
        PhysisCore::from_json(&json).unwrap_or_else(|_| PhysisCore::new())
    }
}

fn walk(dir: &std::path::Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if is_text(&path) {
                out.push(path);
            }
        }
    }
    Ok(out)
}

fn is_text(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("txt" | "md" | "rs" | "py" | "json" | "toml" | "csv" | "log" | "xml" | "yml" | "yaml")
    )
}

trait PersistCore {
    fn persist(&self) -> anyhow::Result<()>;
}

impl PersistCore for PhysisCore {
    fn persist(&self) -> anyhow::Result<()> {
        let json = self.to_json()?;
        std::fs::write(store::nodes_path(), json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::elide_middle;

    #[test]
    fn single_line_collapses_embedded_newlines() {
        // A label carries file content verbatim. Printed raw inside a numbered
        // list, its later lines lose the indentation and read as extra results.
        let label = "/corpus/c.md: # Training notes\nSquat stalled at 120kg.\n\nSleep 6.2h.";
        let out = super::single_line(label);
        assert!(!out.contains('\n'), "must be one line: {out}");
        assert_eq!(
            out, "/corpus/c.md: # Training notes Squat stalled at 120kg. Sleep 6.2h.",
            "runs of whitespace collapse to exactly one space",
        );
    }

    #[test]
    fn short_labels_pass_through_untouched() {
        assert_eq!(elide_middle("a.txt: hello", 110), "a.txt: hello");
    }

    #[test]
    fn long_labels_keep_the_file_name_and_the_start_of_the_content() {
        // The bug: `label.chars().take(100)` spent the whole budget on the
        // shared leading directory, so every search hit printed identically.
        let label = format!(
            "/home/someone/very/deeply/nested/project/directory/that/goes/on/corpus/{}: {}",
            "report_049.txt", "Nozzle temperature dropped below the glass transition point"
        );
        let out = elide_middle(&label, 110);
        assert!(out.chars().count() <= 110, "must respect the budget: {out}");
        assert!(out.contains('…'), "must mark the elision");
        assert!(
            out.contains("report_049.txt"),
            "file name identifies the node: {out}"
        );
        assert!(
            out.contains("Nozzle temperature"),
            "content identifies the node: {out}"
        );

        // Two nodes under the same long directory must not render the same.
        let other = label.replace("report_049.txt", "report_050.txt");
        assert_ne!(elide_middle(&other, 110), out);
    }

    #[test]
    fn multi_byte_labels_do_not_panic_or_split_a_character() {
        // Byte slicing here would panic; the label is counted in chars.
        let label = "café/über/naïve/".repeat(20) + "fine.txt: caffè corretto";
        let out = elide_middle(&label, 60);
        assert!(out.chars().count() <= 60);
        assert!(out.contains("caffè corretto"));
    }
}
