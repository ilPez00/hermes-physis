# Physis Core (`physis-core`)


> **Compression numbers restated 2026-09-12.** Earlier figures (220 → 165, 25%)
> compared two different quantities: the baseline was a *sum of chunk tokens*
> while the physis side counted the *assembled string*, which carries separators.
> Both sides now count the string a model is actually handed. The corrected
> measurement on the same corpus is **253 → 165 tokens, 35%** — the old number
> understated it, because the baseline was undercounted. Caught by a chain test
> asserting `context_tokens <= baseline_tokens`, which a three-document corpus
> inverted.

[![Crates.io](https://img.shields.io/crates/v/physis-core.svg)](https://crates.io/crates/physis-core)
[![Documentation](https://docs.rs/physis-core/badge.svg)](https://docs.rs/physis-core)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

## See the structure inside your data.

Physis finds **what repeats, what differs, and what matters** in large bodies of
information — deterministically — then compiles that structure into compact
context for humans and AI. Offline. No API key. No cloud.

```text
30 DOCUMENTS
  ↓ PHYSIS
3 RECURRING PATTERNS · 1 DIFFERENCE · 1 CONTRADICTION
  ↓
compiled context  — measured 35% smaller than conventional retrieval
```

> **What that 25% is, and is not.** It is a real token count from an actual run
> on `benchmarks/ground-truth`. It is **not** evidence that the *selection* is
> good: that corpus scores identically under a real sentence transformer and
> under a random-projection lexical hash (repeat 100%, anomaly 100%,
> contradiction 50%, compression ~24% either way), because its repeat families
> are lexically near-identical. A benchmark whose score does not move when
> retrieval quality moves is not measuring retrieval quality.
>
> For a measurement that *does* discriminate, see
> [`benchmarks/retrieval`](https://github.com/ilPez00/physis-skill) in the
> superproject: questions asked in a developer's words about code written in a
> compiler's words, scored `hit@5` — **0.57 semantic vs 0.14 lexical, +0.43**.

### Install

```sh
# Not on crates.io. From git, with the ONNX embedder capability:
cargo install --git https://github.com/ilPez00/physis-core \
      --features cli,embed-onnx --locked physis-core
```

`embed-onnx` is **not** a default feature. Without it the embedder cascade has
no model to try and resolves to random projection — a lexical hash that fails
the semantic self-test by design, and says so on stderr.

### Run it (one command, fully offline)

```sh
# Answers from your documents, with citations that cannot be invented:
physis-core notebook --corpus ./my-docs --query "what changed in the spec" --budget 400

# Let the table draft and spend a model only on the gaps:
physis-core notebook --corpus ./my-docs --query "..." --draft

physis-core demo --dir examples/demo-corpus --query "the pump"
physis-core context --corpus examples/demo-corpus --query "what maintenance is scheduled" --budget 400
physis-core benchmark
physis-core studio --port 3000
```

### The three demonstrations
1. **Difference & Repetition** — a corpus collapses into a structural map:
   repeat families, anomalies, contradictions, a reproducibility hash. Deterministic.
2. **Context compiler** — fixed-budget structural context over a corpus, with the
   compression over conventional retrieval **measured from the actual run**.
3. **Small model + n-gram table + Physis** — every component is replaceable via
   `model_provider` / `ngram_table` / `tokenizer`; a real offline decoder backend
   drives the pipeline at ~1 MB. Interchangeability is tested (`Model A + Table
   A/B`, `Model B + Table A/B`).

Full playbooks in [`DEMO.md`](DEMO.md) and [`SHIP_PLAN.md`](SHIP_PLAN.md).

---

> ## STATUS: measured limits, growing capabilities
>
> Filing-certification: **NO** — every scorer at chance on real misfilings (cosine 0.519, constraint 0.536, ancestry 0.534); see `research/perspective-discovery/FINAL_REPORT.md`.
> Cross-embedder discovery: **NO** — ARI ~0.10 across embedders.
> Filing-proposal: **YES, measured** — top-3 0.712 vs permuted-label null 0.136 and old→new lookup 0.368; see [`propose`](https://docs.rs/physis-core/latest/physis_core/propose/).
> A shortlist is not a verdict — triage for a person to pick from, never certification.
> Geometry did not improve — returning three candidates instead of one verdict is the entire difference.
>
> Capability index:
> - [`system_one`](https://docs.rs/physis-core/latest/physis_core/system_one/) — typed decision heads (`rule | pattern | count-table | lexical | prototype | knn | ngram | Jev | LLM | human`) on one calibrated contract, plus the loop that compiles a repeated expensive judgement into the cheapest rung that expresses it. Measured: 54.8% of strong-rung escalations removed at **zero** accuracy cost on clean labels; a shuffled-label control promotes **nothing**; a family nothing can compile stays 100% on the strong rung. Architecture: `docs/SYSTEM_ONE_SEMANTIC_JIT.md`.
> - [`epistemic`](https://docs.rs/physis-core/latest/physis_core/epistemic/) audit trail — competing interpretations with replayable provenance.
> - [`hybrid BM25+RRF retrieval`](https://docs.rs/physis-core/latest/physis_core/rag/) — dependency-free Okapi BM25 fused with cosine.
> - [`structural transform algebra`](https://docs.rs/physis-core/latest/physis_core/transform/) — homomorphism engine with TraceStep-gated apply; reasoning notes in `docs/TRANSFORM_REASONING.md` (parent).
> - [`n-gram embedding scaffold`](https://docs.rs/physis-core/latest/physis_core/embed_ngram/) — collocation vocabulary, not a sense inventory.
> Full controls and numbers in `research/perspective-discovery/FINAL_REPORT.md`.
> Honesty preserved: limits above are measured, capabilities below are tested primitives.

**The lean, high-performance epistemic reasoning engine extracted from the Physis intelligence system.**

> *“An engine that maintains competing interpretations of reality, evaluates their coherence with observations, processes, evidence, and outcomes, and preferentially retains interpretations that continue to work.”*

---

## Table of Contents

- [Overview & Core Epistemic Thesis](#overview--core-epistemic-thesis)
- [Key Capabilities & Innovations](#key-capabilities--innovations)
- [Architectural Boundary: Core vs. Pro](#architectural-boundary-core-vs-pro)
- [Architecture & Epistemic Pipeline](#architecture--epistemic-pipeline)
- [The Semiotic Grid](#the-semiotic-grid)
- [Module Catalog](#module-catalog)
- [Installation & Cargo Features](#installation--cargo-features)
- [Determinism & Reproducibility](#determinism--reproducibility)
- [Cell Linkage](#cell-linkage)
- [Proposing a Filing](#proposing-a-filing)
- [Coverage Impact](#coverage-impact)
- [Epistemic Revision: the Delta Engine](#epistemic-revision-the-delta-engine)
- [System-One Semantic Heads (the cheap rung)](#system-one-semantic-heads-the-cheap-rung)
- [Rust API Usage & Code Examples](#rust-api-usage--code-examples)
  - [1. Competing Hypotheses & Evidence Attestation](#1-competing-hypotheses--evidence-attestation)
  - [2. Truth Maintenance & Contradiction Resolution](#2-truth-maintenance--contradiction-resolution)
  - [3. Epistemic Audit Stream & Historical Replay](#3-epistemic-audit-stream--historical-replay)
  - [4. Token-Fixed Budget RAG](#4-token-fixed-budget-rag)
  - [5. Unsupervised Ontology Gap Discovery](#5-unsupervised-ontology-gap-discovery)
- [CLI Reference](#cli-reference)
- [Embedded Studio Web Workbench](#embedded-studio-web-workbench)
- [Publishing to Crates.io & Integration](#publishing-to-cratesio--integration)
- [License](#license)

---

## Overview & Core Epistemic Thesis

Mainstream AI knowledge graphs, retrieval pipelines, and vector databases operate on naive assumptions:
1. **Premature Convergence**: They collapse ambiguous observations into a single winning embedding or statement.
2. **Destructive Updates**: Contradictory evidence either overwrites prior truth or creates silent hallucination loops.
3. **Static Semantics**: Ontologies are treated as frozen taxonomies, incapable of discovering operational gaps.
4. **Epistemic Amnesia**: They lack an immutable audit log of *why* an assertion was believed, *what* assumptions supported it, and *how* confidence evolved over time.

`physis-core` replaces these flaws with a **dynamic truth-maintenance and coherence-seeking architecture**:

- **Active Hypothesis Competition**: Multiple mutually exclusive interpretations are maintained in parallel.
- **Multidimensional Coherence**: Hypotheses are scored across 5 orthogonal axes (`Semantic`, `Ontological`, `Logical`, `Empirical`, `Process`).
- **Non-Destructive Tension**: Contradictions are explicitly modeled and preserved as first-class entities with contextual preferencing.
- **Time-Machine Replay**: An append-only epistemic event trail allows instantaneous deterministic reconstruction of beliefs at any prior timestamp $t$.
- **Evolutionary Feedback**: Contextual quality feedback loops penalize cells that generate false predictions, dynamically tuning subsequent classifications.

---

---

## Key Capabilities & Innovations

| Feature | Description | Implementation |
|---|---|---|
| **Competing Hypotheses** | Parallel candidate interpretations with explicit assumption tracking, prediction verification, and Bayes-like survival. | `physis_core::hypothesis` |
| **Semiotic Grid (5×14)** | 70 canonical axes mapping 5 philosophical domains across 14 operational modes with sub-domain facets. | `physis_core::classify` |
| **Truth Maintenance System** | Explicit conflict modeling between contradictory claims with confidence-weighted tension and contextual override. | `physis_core::contradiction` |
| **Epistemic Audit & Time Machine** | Cryptographically chained audit stream with point-in-time state reconstruction. | `physis_core::epistemic` |
| **Structured Explanations** | Generation of structured reports detailing supporting/contradicting evidence, historical precedents, and fitness breakdowns. | `physis_core::explanation` |
| **Ontology Gap Discovery** | Unsupervised semantic clustering over unclassified text to propose new domain/mode entries automatically. | `physis_core::discovery` |
| **Fixed-Token Budget RAG** | Strict token-budget bounded recall with Maximal Marginal Relevance (MMR) diversity discounting. | `physis_core::rag` |
| **Vault & History Ingest** | Extract structured knowledge nodes from Markdown vaults, Git history, Netscape bookmarks, browser history, and OPML feeds. | `physis_core::vault`, `physis_core::history` |
| **Exchange-Parts Node Editing** | Swap node content while preserving ID, verdict, edges, and provenance (`cell_pin` hybrid semantics). | `physis_core::core` |

---

## Architectural Boundary: Core vs. Pro

> **Guiding Principle**: *Core stays Core (the pure, dependency-light epistemic reasoning engine), and Pro stays Pro (the enterprise industrial monitoring and operational intelligence suite).*

| Dimension | `physis-core` (Open Source Engine) | `physis-pro` (Industrial Suite) |
|---|---|---|
| **Primary Focus** | Epistemic truth maintenance, coherence evaluation, competing hypotheses | Industrial shop-floor telemetry, backoffice automation, multi-tenant deployment |
| **Ontology Engine** | 70 canonical semiotic grid cells (5 Domains × 14 Modes) + 33 domain ontologies | Extended 370+ industrial, machine process, and agent workflow domains |
| **Conflict & Truth** | First-class `Contradiction` tracking, non-destructive polarity, temporal replay | Shop-floor anomaly escalation, quality failure loop, automated arbitration |
| **State Persistence** | Lean, dependency-light in-memory or single JSON snapshot (`~/.physis-core/`) | High-performance durable Sled DB + Cloud Spanner Graph mirror |
| **Multi-Tenancy** | Single session / embedded in-process | Isolated per-tenant `RuntimeState` mapped via `X-Physis-User` header |
| **Hardware & IoT** | Model-agnostic text and vector embeddings | MQTT, Modbus TCP/RTU, Serial, OPC-UA machine adapters |
| **Multimodal Sensory** | Extensible `VectorEmbed` trait (RandomProjection, ONNX) | Real-time `AuraFrame` sensory bus, Whisper-large voice, CLIP visual features |
| **LLM Integration** | Token-fixed budget retriever (MMR RAG) | Dynamic LLM Coherence Harness, multi-provider cascade, auto-revision loops |
| **User Interface** | Lightweight embedded Axum studio (`physis-core studio`) | Full glassmorphic Operations Console, Gantt scheduler, OEE dashboards |

---

## Architecture & Epistemic Pipeline

```text
                               ┌─────────────────────────────┐
                               │    Raw Observations / Text  │
                               └──────────────┬──────────────┘
                                              │
                                      [Vector Embedder]
                                              │
                                              ▼
┌───────────────────────────┐      ┌─────────────────────────────┐      ┌───────────────────────────┐
│     Semiotic Grid (70)    │ ◄─── │       Cell Classifier       │ ───► │  Unsupervised Discovery   │
│   5 Domains × 14 Modes    │      │  (Nearest-Centroid Scoring) │      │  (Gap Analysis & Cluster) │
└───────────────────────────┘      └──────────────┬──────────────┘      └───────────────────────────┘
                                                  │
                                                  ▼
                                   ┌─────────────────────────────┐
                                   │         PhysisCore          │
                                   └──────────────┬──────────────┘
                                                  │
                ┌─────────────────────────────────┼─────────────────────────────────┐
                ▼                                 ▼                                 ▼
┌───────────────────────────────┐ ┌───────────────────────────────┐ ┌───────────────────────────────┐
│     Competing Hypotheses      │ │     Contradiction Engine      │ │    Epistemic Audit Stream     │
│  - Assumption tracking        │ │  - Polarity tension           │ │  - Append-only event log      │
│  - Multidimensional fitness   │ │  - Contextual resolution      │ │  - Time-machine state replay  │
│  - Empirical predictions      │ │  - Non-destructive conflict   │ │  - Root-cause explanation     │
└───────────────────────────────┘ └───────────────────────────────┘ └───────────────────────────────┘
                │                                 │                                 │
                └─────────────────────────────────┼─────────────────────────────────┘
                                                  │
                                                  ▼
                                   ┌─────────────────────────────┐
                                   │ Contextual Quality Feedback │
                                   │   & Reinforcement Penalty   │
                                   └─────────────────────────────┘
```

---

## The Semiotic Grid

The foundational taxonomy divides knowledge across **5 Ontological Domains** and **14 Semiotic Modes**:

| Domain | Philosophical Meaning | Canonical Focus |
|---|---|---|
| **Techne** | Craft, engineering, instrumentation | Tools, physical parameters, mechanics, code, hardware |
| **Episteme** | Scientific knowledge, causal models | Theories, empirical proofs, equations, verification |
| **Phronesis** | Practical wisdom, situational prudence | Operational decisions, safety trade-offs, risk management |
| **Polis** | Collective governance, organizational systems | Teams, contracts, regulatory compliance, backoffice workflows |
| **Soma** | Physical embodiment, biological state | Sensory streams, machine health, thermal profiles, ergonomics |

Cross-referenced across **14 Operational Modes**:
`Substance`, `Form`, `Relation`, `Quantity`, `Quality`, `Space`, `Time`, `Position`, `State`, `Action`, `Passivity`, `Purpose`, `Process`, `Genesis`.

---

## Module Catalog

- **`classify`**: Computes cosine proximity against domain/mode cell centroids with quality-penalty adjustments.
- **`coherence_dimensions`**: Five-factor coherence profiles and composite fitness aggregation.
- **`coherence_query`**: Declarative queries over coherence graph nodes, verdicts, and thresholds.
- **`contradiction`**: Truth-maintenance conflict records, tension weights, and non-destructive resolution states.
- **`core`**: Central `PhysisCore` orchestrating nodes, edges, hypotheses, Dreaming engine, and memory snapshots.
- **`discovery`**: Density-based ontology gap detection and new domain/mode proposal generation.
- **`embed`**: Zero-dependency deterministic `RandomProjectionEmbedder` and extensible `VectorEmbed` trait.
- **`embed_onnx`**: Optional ONNX runtime integration for high-accuracy embedding models (`all-MiniLM-L6-v2`, `bge-small`, etc.).
- **`epistemic`**: Immutable event sourcing for beliefs, hypothesis states, and temporal replay reconstruction.
- **`explanation`**: Structured explanatory justifications with supporting/contradicting evidence chains.
- **`history`**: Multi-format personal history parsers (Netscape HTML bookmarks, Chrome/Firefox JSON history, OPML, chat JSONL).
- **`hypothesis`**: Competing hypothesis data structures, evidence polarities, and revision histories.
- **`ontology`**: Embedded loader for 33 built-in domain ontologies (human grid, machine process, AI agents, office operations).
- **`praxis`**: Behavioral tracking records with success/inert/failure feedback loops.
- **`propose`**: Filing **proposal** — learns from filings a person has confirmed and returns the few cells worth looking at first. Top-3 0.712 against a permuted-label null of 0.136. It is triage, not certification; see the status notice.
- **`process`**: Industrial and operational state machines, task sequences, and cycle tracking; measurements flag their own deviation from a constraint's `[min, max]` band with a normalized 0..1 severity.
- **`provenance`**: Cryptographic SHA-256 provenance chains connecting source data to final inferences.
- **`quality`**: Quality feedback tracker with cell-level *and* per-agent penalties/boosts, plus contextual fitness weighting — an agent that keeps producing bad output for a domain gets demoted the same way a cell does.
- **`rag`**: Fixed-budget token retrievers with BPE-style approximate tokenization and diversity filtering.
- **`vault`**: Knowledge vault readers for Markdown hierarchies (frontmatter, headings) and Git log streams.
- **`studio`**: Embedded Axum web server providing an interactive browser UI and RESTful HTTP API.
- **`system_one`**: Typed, calibrated decision heads and the loop that compiles them — `WorldSlice + OntologySlice + evidence → Decision<T>{value, distribution, confidence, provenance, cost}`, over nine interchangeable rungs from a rule to a person. Abstains rather than guesses; a head is safe only inside the region its traces came from. 24 integration tests; measured in `examples/experiment54_system_one_heads.rs`.

---

## Installation & Cargo Features

Add `physis-core` to your `Cargo.toml`:

```toml
[dependencies]
# 1. Lean Engine Only (Zero heavy web/ML dependencies)
physis-core = { version = "0.1", default-features = false }

# 2. Complete Engine + CLI + Embedded Studio UI
physis-core = { version = "0.1" }

# 3. Complete Engine + ONNX Real Embeddings Runtime
physis-core = { version = "0.1", features = ["embed-onnx"] }
```

### Feature Flags

| Feature | Default | Dependencies | Purpose |
|---|---|---|---|
| `cli` | **Yes** | `clap` | Standalone CLI binary (`physis-core`) with subcommands. |
| `studio` | **Yes** | `axum`, `tokio` | Embedded Web GUI workbench and REST API server. |
| `embed-onnx` | No | `ort`, `tokenizers` | Hardware-accelerated ONNX semantic embeddings. |
| `http` | No | `ureq` | The oracle leg — an OpenAI-compatible endpoint as the strong rung in a benchmark or a cascade. |
| `head-regex` | No | `regex` | Regex rules for `system_one::heads::PatternHead`. Literal/prefix patterns need no feature; without it a `Regex` rule reports itself unavailable instead of silently matching nothing. |

---

## Determinism & Reproducibility

Several parts of this engine are consumed by pipelines that need the *same
input to produce the same output*, run after run — classification, ontology
gap discovery, and anything that clusters or ranks by embedding similarity.
The guarantees, and their limits, are stated here explicitly because the
limits are easy to get wrong.

### What is guaranteed

| Guarantee | Since | Notes |
|---|---|---|
| `OntologyLoader::classification_domains()` yields entries in a **stable, total order** | 0.1.15 | Sorted by `(name, domain, mode)`. Before 0.1.15 it chained three `HashMap::values()` iterators, and Rust seeds `HashMap`'s hasher randomly **per process** — so the order changed on every run. See below. |
| `RandomProjectionEmbedder` is deterministic | — | Seeded projection; reproducible and coarse. Also selectable via `PHYSIS_EMBEDDER=random-projection`. |
| `OnnxEmbedder` output is deterministic | — | Measured bit-identical for the same input across processes **and across intra-op thread counts** (`examples/probe_embedding_determinism.rs`). |
| Epistemic event replay | — | Append-only trail; belief state at any prior timestamp reconstructs exactly. |

### The 0.1.15 ordering fix — worth reading if you use this crate

`classification_domains()` returned entries in `HashMap` iteration order.
That order is randomized per process by design in Rust (SipHash with a
random seed, as DoS hardening). Any downstream computation sensitive to
input order — centroid accumulation, train/test splits, tie-breaking
between near-equal similarity scores — therefore produced **different
results on every run from identical data and identical model weights**.

This was subtle enough to be misdiagnosed twice in our own research track
as floating-point nondeterminism in the ONNX runtime. It was not. If you
observed unstable classification or discovery output on <= 0.1.14, upgrade
before investigating anything else.

**If you iterate ontology entries yourself**, apply the same discipline:
sort before you fold, and break ties on a stable key rather than relying
on a stable sort over an unstable input order.

### What is *not* guaranteed

- **Cross-embedder stability.** Results are not portable between embedding
  models. Structure derived under one model does not survive a swap to
  another (we measured near-random agreement, ARI ~= 0.10, between MiniLM
  and BGE embeddings of the same corpus, even holding cluster identity
  fixed). Treat the embedder as **part of your ontology's identity**, not
  as an interchangeable backend — pin it, and version your derived
  structures against it.
- **Stability of re-derived clusters under corpus growth.** Anchors
  re-derived from scratch after new data move substantially. Persist
  anchor identity across runs and assign new items to existing anchors;
  do not re-derive and assume continuity.

### `OnnxConfig::intra_threads`

```rust
use physis_core::embed_onnx::{OnnxConfig, OnnxEmbedder, PoolingStrategy};

let embedder = OnnxEmbedder::with_config(&OnnxConfig {
    dim: 384,
    model_dir: Some("models".into()),
    pooling: PoolingStrategy::Mean,
    intra_threads: Some(1), // opt-in; `None` (default) uses all cores
    ..OnnxConfig::default()
});
```

Opt-in ONNX Runtime intra-op thread control, default `None` (unchanged
behavior, all cores). Provided for throughput/latency tuning and for
pinning inference in constrained environments. It is **not** a determinism
knob — this embedder's output was measured bit-identical at every thread
count, so setting it to `Some(1)` will not change your results, only your
performance.

---

## Cell Linkage

`physis_core::linkage` answers a question the classifier alone does not:
**which `(domain, mode)` cells does your data actually bridge, and through
which items?**

```rust
use physis_core::classify::CellClassifier;
use physis_core::linkage::LinkageGraph;

let classifier = CellClassifier::build(&ontology, &embedder);
let graph = LinkageGraph::build(&classifier, &embedder, corpus_texts);

for link in graph.strongest(10) {
    println!("{}/{} <-> {}/{}  ({} bridges)",
             link.a.0, link.a.1, link.b.0, link.b.1, link.bridge_count);
}

// Only the links a single-label classification cannot represent:
for link in graph.cross_domain() { /* ... */ }
```

Each text contributes exactly one bridge, between its top-scoring and
second-scoring cell. **No threshold, no `k`, no random seed**, and ties break
on the cell key rather than on iteration order — so the same corpus and the
same embedder always produce the same graph.

### What it deliberately does not do

It does not *discover* cells. It takes the ontology's hand-authored cells as
given, because they are the only thing measured to be stable: they cannot
drift, and they are identical on every run.

That restraint is a finding, not modesty. Seven structurally distinct
mechanisms for deriving stable groupings from embedding geometry were tried
and all seven failed on real data — margin/silhouette gating, three
kNN-consistency variants, cross-embedder corroboration, density peaks,
cross-embedder split agreement, capacity-constrained training loss, and plain
k-means. Two specific limits are worth stating outright, because both are easy
to assume away:

- **Re-derived clusters do not survive corpus growth.** Adding 25% more data
  replaced half the anchors (58% identity overlap) and destroyed ~80% of the
  structure. Persist anchor identity; do not re-derive and assume continuity.
- **Nothing survives an embedder swap.** Agreement between MiniLM- and
  BGE-derived structure on the same corpus is near-random (ARI ~0.10) even
  holding cluster identity fixed. Pin your embedder.

### Reading the output

The signal is aggregate, not per-item. A cell pair that recurs as many
different items' runner-up is meaningfully linked; a single bridge is noise.
No claim is made that any individual item is "genuinely cross-cutting" — an
earlier thresholded version of this calibrated a similarity delta and
degenerated into flagging 98% of entries, a rate that carries no information
beyond "domains overlap".

---

## Proposing a Filing

`physis_core::propose` is the only mechanism here that survived a
construction-matched control, and it does so by asking a narrower question than
the rest of this crate originally asked. It does **not** decide whether an entry
belongs somewhere. It learns from filings a person has already confirmed and
hands back the few cells worth looking at first.

```rust
use physis_core::embed::RandomProjectionEmbedder;
use physis_core::propose::{embed_entry, Proposer, DEFAULT_HINT_WEIGHT};

let embedder = RandomProjectionEmbedder::new(256);

// Filings a person already confirmed. Real use accumulates these as they work.
let confirmed = [
    ("HEAL", "REST", "Sleep Hygiene", vec!["circadian".to_string()]),
    ("STUDY", "LEARN", "Spaced Repetition", vec!["recall".to_string()]),
    ("BOND", "CREATE", "Team Charter", vec!["agreement".to_string()]),
];

let proposer = Proposer::from_decisions(confirmed.iter().map(|(d, m, name, hints)| {
    (*d, *m, embed_entry(name, hints, &embedder, DEFAULT_HINT_WEIGHT))
}));

// A new entry: show the person a shortlist, let them choose.
let entry = embed_entry("Nap Protocol", &["rest".to_string()], &embedder,
                        DEFAULT_HINT_WEIGHT);
for p in proposer.propose(&entry, 3) {
    println!("{}  {:.3}", p.cell(), p.score);
}
```

Runnable as `cargo run --example propose_readme`, so the snippet cannot rot.

> **The output of that example is semantically meaningless, deliberately.**
> `RandomProjectionEmbedder` is deterministic and dependency-free but carries **no
> semantics** — it hashes tokens into a fixed basis. Run as written, "Nap
> Protocol" is proposed into `STUDY/LEARN` rather than `HEAL/REST`. The example
> shows the *plumbing*; the accuracy figures on this page were measured with a
> real sentence embedder. Use the `onnx` feature for anything you intend to
> believe.

**Read the shortlist as triage.** At k = 3 the cell a human actually chose was
present about two times in three on the corpus this was measured against. That
leaves a third where it is absent — which is why this returns candidates for a
person rather than a verdict.

### Two things worth knowing before you rely on it

**The `domain` axis is far stronger than `mode`.** Top-2 containment is 0.837
over five domains against 0.678 over fourteen modes, and the gap is structural:
domain compresses losslessly into four dimensions, while mode is not a
low-dimensional object at all. **Propose the domain confidently and expect the
person to pick the mode.**

**Hints are over-weighted at full strength.** Entries are conventionally embedded
as `name + hints`, but hints are subject-matter vocabulary — they help `domain`
and *hurt* `mode`. [`DEFAULT_HINT_WEIGHT`] is `0.5`, which beat both the name
alone and the full text on both axes, worth +0.054 top-3 over the latter across
24 paired splits. [`embed_entry`] applies it; it costs one extra embedder call.

[`DEFAULT_HINT_WEIGHT`]: https://docs.rs/physis-core/latest/physis_core/propose/constant.DEFAULT_HINT_WEIGHT.html
[`embed_entry`]: https://docs.rs/physis-core/latest/physis_core/propose/fn.embed_entry.html

---

## Coverage Impact

`physis_core::coverage` answers the question [`discovery`](#5-unsupervised-ontology-gap-discovery)
leaves open: of the entries it proposes, **which are worth keeping?**

```rust
use physis_core::coverage::{rank_candidates, uncovered};

// Records the live ontology cannot place confidently.
let gaps = uncovered(&classifier, &records, threshold);

// Which candidates are worth reading, strongest first (ordered by lift).
for (idx, gain) in rank_candidates(&classifier, &candidates, &records, threshold) {
    println!("candidate {idx}: lift {:.3}, newly covers {} records", gain.lift, gain.count());
}
```

### Why the check is an operational question, not a semantic one

Nine mechanisms for certifying discovered structure were tried across a long
research track and all nine failed. Every one asked a **representational**
question — is this split correct, does a lexicon recognise it, do many pairs
agree on it — and answered it with a statistic computed over the same embedding
space that produced the candidate. The geometry kept grading its own homework.

This asks an **operational** one: does adding the candidate make records
classifiable that were not? The corpus is external to whatever produced the
candidate, so the check cannot be satisfied by the geometry agreeing with
itself.

Measured over 5-fold cross-validation on 691 held-out records:

| candidate | records rescued |
|---|---|
| an entry the ontology already has | **0 of 2000 trials** |
| a genuine interpolation | above zero (Welch t = +12.11) |

It is not fooled by a candidate that adds nothing, and needs neither a human
nor a lexicon to say so. In that run it reduced **2000 candidates to 141**
worth reading.

### Two measures: one filters, the other ranks

`CandidateGain` reports the same event twice, because the two were measured
separately and do different jobs.

| field | what it is | job |
|---|---|---|
| `count()` | records that cross the confidence threshold | **filter** — the measure validated at t = +12.11 above |
| `lift` | `Σ max(0, cos(candidate, record) − live(record))`, threshold-free | **rank** — what `rank_candidates` sorts on |

Ranking by lift was measured against a 10% hold-out of real ontology entries,
1,638 proposals scored over 6,234 operational records:

| ordering | precision@25 | @50 | @100 | exact p @25 |
|---|---|---|---|---|
| **`lift`** | **0.480** | 0.400 | 0.360 | **2.3e-5** |
| `count()` | 0.320 | 0.300 | 0.200 | 1.2e-2 |
| proposal popularity | 0.160 | 0.140 | 0.180 | 0.43 |
| the proposer's own geometry | 0.200 | 0.140 | 0.120 | 0.23 |
| how empty that region of the ontology is | 0.120 | 0.140 | 0.160 | 0.66 |
| random | 0.120 | 0.140 | 0.130 | 0.66 |

Base rate 0.132. Twelve of the first twenty-five rows point at a real missing
concept, against 3.3 expected. The four controls sit on the random floor at the
head of the list, so the ordering is coming from the records — **the ontology's
own geometry ranks its own holes at chance.**

**Why the continuous form matters more than it looks.** The threshold rule goes
nearly inert on operational text. Records written by a running operation already
match the live ontology strongly (measured median best-entry cosine 0.741), and
a candidate interpolated between two existing entries can rarely beat *every*
known entry for *any* record — at that median only 272 of 1,638 candidates
rescued even one. The 5-fold run above looked healthier because its records were
held-out ontology *entries*, which sit further out and leave the bar reachable.
Lift has no bar and degrades smoothly.

Both measures kill a duplicate identically: the live score is already a maximum
over every entry, so a copy of one cannot exceed it, and its lift is exactly
`0.0` by construction.

**Read the head, not the tail.** Precision decays 0.480 → 0.360 → 0.193 across
k = 25 → 100 → 400 and is inside the base rate by 400. This orders a worklist of
about a hundred; it does not sort a pile into good and bad.

### Coverage is not correctness

A candidate that swallows records into the *wrong* cell scores exactly like one
capturing a real gap — both make the records classifiable. Distinguishing them
needs labels, and is precisely the question those nine mechanisms failed to
answer.

**Use this to shrink the pile, then have a person read what survives.** It is a
filter and an ordering, not an approver. Pair it with a threshold chosen against
your observed score distribution: if nothing is uncovered, `count()` is
uniformly zero — `lift` still ranks, but only among candidates the records
actually pull on.

Ranking does not soften the caveat. The ordering was validated against
**missing** concepts (a hold-out), never against **wrong** ones. Asked instead
to score entries against the cell they were actually filed in, the same cosine
machinery detects real misfilings at **AUC 0.410**, chance inside the confidence
interval. It finds gaps; it cannot find mistakes.

Two further limits:

- **It ranks gaps the records talk about.** The hold-out concepts that validated
  it carry heavy corpus traffic (median max-record cosine 0.793). A concept the
  ontology is missing *and* the operation never writes about produces no lift
  and cannot surface here, by construction.
- **Not every candidate is rankable.** 1,236 of 1,638 proposals (75%) lifted any
  record at all; the rest tie at zero and fall back to candidate order.

---

## Epistemic Revision: the Delta Engine

When a node mutates, `delta_engine` evaluates the cascade in an isolated
shadow frame and reports what *would* change; the caller commits. Since
0.1.22 the engine also carries the epistemic-revision track — patterns taken
from Atlas (Ripple / AGM / adjudication) and Graphiti (temporal legs, ingest
discipline) as behavior and tests, never as dependencies. Every item ships
behind a named gate test; every claim keeps its null.

### No-perturbation invariant (A7)

A zero `EmbeddingShift` carries no information: the wave returns empty,
fitness stays bit-identical, zero transitions. Gate test:
`zero_shift_wave_leaves_fitness_untouched`.

### Revision selection by declared dependency (A2)

Which hypotheses are revised is decided by the **DependsOn closure** of the
mutated node (`EvaluationContext::depends_on_walk` — BFS shallow-first,
justification-hop counting, cycle recording, depth cap 5, node cap 5000),
not by the breadth wave. A shared cell pin, a label prefix, or a similar
embedding no longer pulls a hypothesis into revision on mere arrival.
Graphs that declare no DependsOn edges fall back to the breadth-affected
selection, and the report says so (`RevisionWalk::fallback_breadth_used`).
Gate test: `midchain_revision_revises_exact_dependents`.

### Named fitness weights + per-term breakdown (A6)

The composite weights are published frozen constants
(`FITNESS_WEIGHT_SEMANTIC_FIT` 0.20, `FITNESS_WEIGHT_ONTOLOGICAL_FIT` 0.15,
`FITNESS_WEIGHT_LOGICAL_CONSISTENCY` 0.15, `FITNESS_WEIGHT_EMPIRICAL_SUPPORT`
0.25, `FITNESS_WEIGHT_PREDICTIVE_SUCCESS` 0.25, plus the penalty schedules),
and `Hypothesis::fitness_term_breakdown()` reports `weight × term` per
recompute; the contributions sum to the fitness wherever the `[0, 1]` clamp
does not bite. Tuning is a separate scored config, never an in-place edit.
Gate test: `fitness_recompute_reports_term_breakdown`.

### Adjudication routing (A5)

A proposed demotion is **routed, not blindly applied** (`route_transition`):

| route | when | effect |
|---|---|---|
| `AutoApply` | ϵ < Δ ≤ ϵ + 0.15 | applied as before; decision recorded |
| `StrategicReview` | Δ > ϵ + 0.15 | **proposed, not applied** — status unchanged, `ResolutionStatus::Open`, rationale recorded |
| `CoreProtected` | status `Certified` | flagged, never auto-demoted |

Every proposed demotion carries an `AdjudicationDecision` (route, proposed
status, Δ, resolution, rationale) on `OntologyDeltaReport::adjudications`
and in the hypothesis revision trail — rationale record only; the
approve / reject / adjust / synthesize queue is a later item. The
degradation itself is recorded fact on every route: evidence lands, fitness
absorbs it. Gate tests: `large_delta_routes_to_open_with_rationale`,
`certified_is_core_protected`,
`small_delta_auto_applies_with_decision_recorded`,
`route_transition_boundary_table`.

### Invalidate-don't-delete (G7) + the expired_at leg (G1)

Superseded hypotheses are never removed:
`hypothesis::current_hypotheses` excludes them from live queries,
`hypothesis::hypotheses_including_history` keeps them readable with revision
timestamps. `TemporalValidity` gains the system-invalidation leg `expired_at`
(distinct from `valid_until`, which is when the claim stopped being *true*)
— fields only; query semantics are a later item. Gate tests:
`superseded_items_stay_queryable_for_history`, `temporal_triple_serialises`.

### Temporal dream: the past re-proposes itself (23.6)

Retired branches are retained (G7), so the time machine can do more than
replay them — `dream::dream_over_history(&trail, &mutations, lookback)`
replays in assertion order (G3) and returns `RetrospectiveProposal`s:

- **ReactivateHypothesis** — a superseded/failed/inert branch whose pattern
  re-presented itself in later observations;
- **RestoreConnection** — a severed DependsOn connection whose target has
  since re-confirmed;
- **RetireConnection** — a connection whose target keeps contradicting.

Every proposal cites the historical event/mutation ids that motivated it;
the dream takes the trail by shared reference and **cannot write** — the
caller decides. Many things already happened, and may re-present
themselves; the dream is how the past speaks. Clocks live only in the
trail/replay layer: coherence, classification, and hypothesis state stay
timeless. Gate tests: `dream_proposes_reactivation_from_retained_branches`,
`dream_proposes_restoring_severed_connections`,
`dream_proposes_retiring_repeatedly_contradicted_connections`,
`dream_stays_silent_without_reconfirmation`, `dream_never_writes`.

**Honesty constraints carry.** The machinery proposes, carries, and defers;
it does not verify. Nothing here upgrades the measured numbers (cosine
0.519 ≈ chance on real misfilings; propose top-3 0.712 proposes, never
verifies).

---

## System-One Semantic Heads (the cheap rung)

`classify`, `propose` and the oracle leg all answer questions. This module is about
the ones that stopped being questions: a distinction drawn in the same region of the
corpus, often enough that re-deriving it is waste. It gives every backend one typed
contract, and a loop that moves a distinction down a cost ladder — **only** when the
move has been shown not to cost the task.

```text
WorldSlice + OntologySlice + [HeadEvidence]  →  Decision<O>

Decision<O> {
    value:          Option<O>,       // None = abstain
    distribution:   Vec<Mass<O>>,    // ordered, normalised
    confidence:     f32,             // what a gate thresholds
    raw_confidence: f32,             // before calibration, kept to compare
    provenance:     Provenance,      // head id, kind, contract, derivation, traces
    cost:           Cost,            // micros, RAM, tokens, USD, and its source
}
```

Nine interchangeable rungs, cheapest first, all of them real implementations:

| rung | how it answers | built on |
|---|---|---|
| `rule` | a predicate over typed state, learned as a split that beats the region's base rate | — |
| `pattern` | literal/prefix cues from discriminating tokens; regex behind `head-regex` | `regex` (optional) |
| `count-table` | `P(label ⁄ last tokens)` from a backed-off table | `ngram_table` |
| `lexical` | Okapi BM25 over per-label documents | — |
| `prototype` | nearest label centroid | same cosine `classify` scores cells with |
| `knn` | weighted k-NN over remembered cases | — |
| `ngram` | candidate labels scored by a `ModelProvider` (SLM or n-gram decoder) | `model_provider` |
| `jev` / `llm` | a System-One-shaped oracle, or any `DecisionOracle` (chat adapter behind `http`) | `oracle` |
| `human` | queues the case, then distils the answers into a count table | `ngram_table` |

**The ladder is only half of it.** `system_one::jit` is the other half:

```text
novel task → expensive rung → traces + validated outcomes
           → recurring distinction?          support + a base-rate control
           → which distinctions matter, where?  information gain (bits) × semantic-Δ priority
           → cheapest rung that expresses it     rule → pattern → count → prototype → lexical
           → fit, calibrate on a held-out tail    conformal selective gate
           → promote only on evidence             paired bootstrap, ε = 2pp, decided on the lower bound
           → monitor, fall back when it moves     audit window → retire → strong rung
```

Traces are `observe::Observation`s (`source = system_one`); invalidation runs
`delta_engine`'s own propagation plus its declared-dependency walk and intersected
with a per-token dependency index, so **only the region a delta moved is
recomputed**; a head's standing is `quality`'s per-agent loop, not a private score.

### What is measured

`cargo run --release --example experiment54_system_one_heads` — 3000 work orders,
three families, five arms, seeded. A `typed` family a rule can compile, an `order`
family whose two classes carry **the same bag of words** (nothing lexical can see
it), and a `hidden` family no head is given the key to.

| arm | accuracy | escalations avoided | cheap answers | oracle $ |
|---|---|---|---|---|
| every request to the strong rung | 0.9433 | 0.0% | 0 | 0.0510 |
| one synthesis, deployed ungated | 0.9507 | 0.0% | 0 | 0.0520 |
| **the full loop** | **0.9713** | **38.8%** | 1163 | **0.0414** |
| the loop, gate disabled | 0.9760 | 52.6% | 1577 | 0.0344 |
| the loop, **shuffled labels** (control) | 0.9537 | 0.0% | 0 | 0.0612 |
| **the loop vs a noiseless strong rung** | **1.0000** | **54.8%** | 1643 | **0.0333** |

- Against a noiseless strong rung: **same accuracy, CI [0, 0], 54.8% of the
  expensive traffic gone.** That is the answer to the question this module exists to
  ask.
- Against a 5%-noisy strong rung the loop is *more* accurate than its baseline
  (+2.80pp, CI [+1.73, +3.83]) — and that margin is partly independent label noise
  averaging out across many labels per split, which is why the noiseless ablation
  ships beside it. The noiseless run is the honest number.
- The shuffled-label loop **promoted nothing**, the `hidden` family stayed **100%**
  on the strong rung, and a compiled feature's meaning flipping was caught in
  ~100 items and demoted.
- A run prints its verdict and asserts it is the interval's, not a preference.

### What it deliberately does not do

- **No stub backends.** If a rung cannot answer it abstains and says so in its
  provenance; if nothing can express a distinction, the loop says *stays expensive*
  and keeps paying.
- **No promotion on training accuracy, and no calibration on the data it trained
  on.** The split is chronological and the gate is calibrated on the tail.
- **No network in this module.** The strong rung is the `DecisionOracle` trait;
  `HttpOracle` (feature `http`) is one implementation, a stand-in in tests is
  another, and the benchmark labels it as such in every record.
- **No claim that a compiled rule is a judgement.** A head is a compiled
  distinction, safe only in the region its traces came from — which is why the
  region is in the spec, why a semantic delta retires it, and why drift retires it
  too.

Architecture, borrowed components with licences, the three defects the experiment
found while running it, and the limits that remain: `docs/SYSTEM_ONE_SEMANTIC_JIT.md`.

Pinned by 24 integration tests (public API only) — run
`cargo test -p physis-core --test test_system_one`; library units alongside each
module — `cargo test -p physis-core system_one::`. The measurement, its controls and
its artifact (`benchmarks/results/semantic_jit.json`) are reproducible with
`cargo run -p physis-core --release --example experiment54_system_one_heads`.

---

## Rust API Usage & Code Examples

### 1. Competing Hypotheses & Evidence Attestation

```rust
use physis_core::{
    PhysisCore, RandomProjectionEmbedder, Hypothesis, HypothesisStatus,
    Evidence, EvidencePolarity, Prediction, VectorEmbed,
};

fn main() {
    let embedder = RandomProjectionEmbedder::new(64);
    let mut core = PhysisCore::new();

    // Register competing explanations for a production quality defect
    let emb_a = embedder.embed("Extrusion temperature too low causing delamination");
    let mut hyp_a = Hypothesis::new("Low nozzle temperature", emb_a);
    hyp_a.assumptions.push("Thermistor calibration is accurate".to_string());
    let id_a = core.register_hypothesis(hyp_a);

    let emb_b = embedder.embed("Filament moisture absorption causing steam bubbles");
    let hyp_b = Hypothesis::new("Wet filament spool", emb_b);
    let id_b = core.register_hypothesis(hyp_b);

    // Corroborate hypothesis A with thermocouple measurement
    let ev = Evidence {
        source: "thermal_camera_infrared".to_string(),
        polarity: EvidencePolarity::Supports,
        confidence: 0.95,
        claim: "Melt zone thermal gradient is 18C below target setpoint".to_string(),
        observed_at: Some(chrono::Utc::now()),
        embedding: vec![],
        context: vec!["nozzle_diameter: 0.4mm".to_string()],
    };
    core.hypotheses.get_mut(&id_a).unwrap().add_supporting_evidence(ev);

    // Commit to something falsifiable BEFORE the next run, then record what
    // actually happened. This — not hand-assigning a status — is what moves
    // fitness: `resolve_prediction` writes `Prediction.correct`, which feeds
    // the `predictive_success` term and the falsified-prediction penalty.
    if let Some(h) = core.hypotheses.get_mut(&id_a) {
        h.add_prediction(Prediction::new(
            "Raising the setpoint 18C eliminates delamination on the next run",
        ));

        // ... the next run happens, and it does not ...
        let resolved = h.resolve_prediction(0, "Delamination unchanged at +18C", false);
        assert!(resolved);
        assert_eq!(h.predictions[0].correct, Some(false));

        // Fitness fell on its own; nothing set it by hand.
        // Resolution is write-once — a second call is refused, not applied:
        assert!(!h.resolve_prediction(0, "actually it worked", true));

        // Status changes go through `transition_to`, which records the
        // before/after pair in the revision history.
        h.transition_to(HypothesisStatus::Failed, "prediction falsified", None);
    }

    // Anything still outstanding, with the index `resolve` wants:
    for (idx, pending) in core.hypotheses[&id_a].open_predictions() {
        println!("#{idx} still open: {}", pending.statement);
    }
}
```

### 2. Truth Maintenance & Contradiction Resolution

```rust
use physis_core::{
    PhysisCore, Contradiction, ContradictionParty, ResolutionStatus,
};

fn main() {
    let mut core = PhysisCore::new();

    // Create an explicit contradiction between two sensor claims
    let claim_1 = ContradictionParty {
        source: "sensor_flow_meter_a".to_string(),
        claim: "Coolant line pressure is 4.2 bar (Nominal)".to_string(),
        confidence: 0.88,
        context: vec!["sampled_at_manifold".to_string()],
    };

    let claim_2 = ContradictionParty {
        source: "sensor_pressure_transducer_b".to_string(),
        claim: "Coolant line pressure is 0.8 bar (Cavitation Risk)".to_string(),
        confidence: 0.94,
        context: vec!["sampled_at_impeller".to_string()],
    };

    let conflict = Contradiction::new(claim_1, claim_2, 0.85);
    let conflict_id = core.register_contradiction(conflict);

    // Later: resolve with contextual grounding without deleting the dissenting record
    core.resolve_contradiction(
        &conflict_id,
        ResolutionStatus::ResolvedPreferredB,
        "Transducer B is downstream of clogged line filter; cavitation verified",
    );
}
```

### 3. Epistemic Audit Stream & Historical Replay

```rust
use physis_core::{
    PhysisCore, EpistemicEvent, EpistemicEventType,
};

fn main() {
    let mut core = PhysisCore::new();
    let hyp_id = "hyp-550e8400-e29b-41d4-a716-446655440000";

    // Record belief lifecycle events
    let t0 = chrono::Utc::now();
    core.epistemic_audit.record(
        EpistemicEvent::new(EpistemicEventType::HypothesisGenerated, hyp_id, "Candidate proposed")
            .with_metric(0.50),
    );

    core.epistemic_audit.record(
        EpistemicEvent::new(EpistemicEventType::HypothesisSupported, hyp_id, "Telemetry verified")
            .with_metric(0.92),
    );

    // Replay: Query what the engine believed at timestamp t0
    let snapshot = core.epistemic_audit.replay_state_at(hyp_id, t0);
    assert_eq!(snapshot.status_at_time, "Candidate");
}
```

### 4. Token-Fixed Budget RAG

```rust
use physis_core::{RagCorpus, RagChunk, TokenFixedRetriever, RandomProjectionEmbedder, VectorEmbed};

fn main() {
    let embedder = RandomProjectionEmbedder::new(64);
    let mut corpus = RagCorpus::new();

    corpus.add_chunk(RagChunk::new("doc-1", "Standard operating maintenance procedure for turbine pumps. Check oil level.", embedder.embed("turbine pump maintenance")));
    corpus.add_chunk(RagChunk::new("doc-2", "Emergency shutdown protocol for pressure loss exceeding 2 bar.", embedder.embed("emergency shutdown pressure")));

    let retriever = TokenFixedRetriever::new();
    let query_vec = embedder.embed("pump oil check");
    
    // Retrieve maximum relevant context bounded by 50 tokens
    let result = retriever.retrieve_bounded(&corpus, &query_vec, 50, 0.70);
    println!("Retrieved {} chunks ({} tokens)", result.chunks.len(), result.total_tokens);
}
```

### 5. Unsupervised Ontology Gap Discovery

```rust
use physis_core::{discover, DiscoveryConfig, RandomProjectionEmbedder, OntologyLoader, PhysisConfig};

fn main() {
    let embedder = RandomProjectionEmbedder::new(64);
    let ontology = OntologyLoader::load_all(&PhysisConfig::default());

    let unmapped_corpus = vec![
        "Quantum qubit decoherence in dilution refrigerator".to_string(),
        "Microwave pulse calibration on superconducting transmon".to_string(),
        "Surface code lattice surgery for fault-tolerant logical qubits".to_string(),
    ];

    let config = DiscoveryConfig {
        coverage_threshold: 0.85,
        min_cluster_size: 2,
        max_clusters: 5,
    };

    let report = discover(&unmapped_corpus, &ontology, &embedder, &config);
    println!("Discovered {} candidate domain proposals", report.proposed_domains.len());
}
```

### 6. Process Deviation Detection

```rust
use physis_core::{ProcessConstraint, ProcessCycle, ProcessMeasurement};

fn main() {
    let constraint = ProcessConstraint {
        id: "spindle-temp".into(),
        name: "Spindle temperature".into(),
        metric: "temp_c".into(),
        min_value: None,
        max_value: Some(80.0),
        is_hard_constraint: true,
    };

    let mut cycle = ProcessCycle::default();
    cycle.measurements.push(ProcessMeasurement {
        id: "m-1".into(),
        metric: "temp_c".into(),
        value: 92.0,
        unit: "°C".into(),
        machine_or_source: "cnc-01".into(),
        timestamp: chrono::Utc::now(),
        is_nominal: false,
    });

    // Flags every measurement outside its matching constraint's band.
    let added = cycle.scan_deviations(&[constraint]);
    println!("{added} deviation(s): {:?}", cycle.deviations);
}
```

### 7. Per-Agent Quality Tracking

```rust
use physis_core::{QualityTracker, RandomProjectionEmbedder};

fn main() {
    let mut quality = QualityTracker::new(Box::new(RandomProjectionEmbedder::new(64)));

    // A cell-level penalty (existing) and an agent-level one (new) are tracked
    // separately, so a bad agent in an otherwise-healthy domain gets demoted
    // without punishing every other agent working that cell.
    quality.report_agent_failure("agent-euclid-worker-3");
    quality.report_agent_failure("agent-euclid-worker-3");
    quality.report_agent_success("agent-euclid-worker-3");

    println!(
        "penalty: {:.2}, boost: {:.2}",
        quality.agent_penalties.get("agent-euclid-worker-3").copied().unwrap_or(0.0),
        quality.agent_boosts.get("agent-euclid-worker-3").copied().unwrap_or(0.0),
    );
}
```

---

## CLI Reference

### `physis` — the front door

Installing this crate gives you two executables: `physis-core`, the engine CLI,
and `physis`, a thin front door over whichever Physis edition is present.

```sh
cargo install physis-core     # installs `physis-core` and `physis`
physis -h                     # one help screen covering both editions
```

`physis -h` lists Core's commands and Pro's, marks which side is installed, and
forwards everything else unchanged — `physis classify …` runs `physis-core`,
`physis doctor …` runs `physis-pro`. Subcommand `--help`, exit codes and stdio
are untouched, because the front door `exec`s the target rather than wrapping it.
Names that exist on both sides (`classify`, `scan`, `discover`, `quality`,
`facet`) resolve to Core, so they mean the same thing whether or not Pro is
installed.

```sh
physis upgrade                # what Pro adds, and where to get it
physis web                    # serve the Pro dashboards (needs Pro)
```

Pro is a separate, licensed product. Core does not link it — the two are joined
at runtime by locating the Pro executable on disk — so Core keeps working, and
keeps its Apache-2.0 licence, whether or not Pro is there. The studio shows the
same information under its **Pro / Upgrade** tab, served from `/api/edition`.

### `physis-core`

The `physis-core` CLI exposes full engine capabilities directly to the terminal:

```sh
# 1. Semiotic Classification
physis-core classify "Nozzle temperature dropped below glass transition point"

# 2. Inspect Loaded Ontology Axes & Entries
physis-core ontology --search "thermal"

# 3. Filter Ontology Entries by Facets
physis-core facet --lifecycle OPERATE --agency SELF --kind machine

# 4. Corpus Ingestion into Coherence Graph
physis-core scan /path/to/engineering/vault

# 5. Coherence Similarity Search
physis-core search "bearing fatigue" --limit 5

# 6. Report Asserted Verdict (Reinforcement Signal)
physis-core assert "Extrusion nozzle check" failure

# 7. Execute Dream Cycle (Dissent Replay)
physis-core dream

# 8. Unsupervised Ontology Gap Discovery
physis-core discover /path/to/unclassified/notes --min-cluster 3

# 9. Launch the Embedded Studio Web Workbench
physis-core studio --port 3000 --host 127.0.0.1
```

### `physis-core hypothesis` — the epistemic loop

Everything above classifies. This subcommand is the part that *keeps score*: it
records what you believed, what you predicted would follow, and — crucially —
what actually happened.

```sh
# Register a candidate. Prints the id; the first 8 chars are enough everywhere.
physis-core hypothesis create "Clogs correlate with ambient humidity" --confidence 0.55

# What do we already believe about this? Ranked, dead claims hidden.
physis-core hypothesis query "humidity clog" --limit 5

# Attest evidence, with a polarity and a weight.
physis-core hypothesis evidence a1b2c3d4 "Three clogs, all above 62% RH" --source "line-2 log" --weight 0.7
physis-core hypothesis evidence a1b2c3d4 "One clog at 31% RH" --contradicts --weight 0.4

# Commit to something falsifiable BEFORE you find out.
physis-core hypothesis predict a1b2c3d4 "Next clog occurs above 60% RH" \
    --expected "RH > 60 at the timestamp of the next clog event"

# What did I predict and never check?  Oldest first.
physis-core hypothesis open --older-than 14

# Record the outcome. Fitness moves. This is the step that makes the rest real.
physis-core hypothesis resolve a1b2c3d4 0 "Clogged at 34% RH" --wrong

physis-core hypothesis explain a1b2c3d4     # full structured report
physis-core hypothesis transition a1b2c3d4 failed --reason "prediction falsified twice"
```

**Ids are prefixes.** Every command that takes an id accepts any unambiguous
prefix of it, because that is what `list` and `query` print. An ambiguous prefix
is an error that shows you the candidates; it never silently picks one.

**`resolve` requires a verdict.** Exactly one of `--correct` or `--wrong` — the
command refuses if you pass neither or both, rather than defaulting. A default in
either direction would let a falsified prediction be recorded as successful by
omission, which is the one outcome this command exists to prevent.

**Resolution is write-once.** Re-resolving an already-resolved prediction is
refused. A record whose outcome can change on a second call is not a record.

**What resolving actually moves.** `Prediction.correct` feeds `predictive_success`,
which is 25% of the composite fitness, plus a separate penalty for falsified
predictions:

```text
fitness 0.500 -> 0.225    after one prediction resolved --wrong
fitness 0.500 -> 0.625    after one prediction resolved --correct
```

#### A caveat on `query` relevance scores

`query` ranks by cosine similarity in the embedding space, and **the default
embedder is `RandomProjectionEmbedder`** — hashed bag-of-words with no trained
weights. Its absolute scores are not meaningful: unrelated sentences routinely
score above 0.8. In one measured case, the query *"stripe payment webhooks in the
praxis backend"* scored **0.830** against a claim about `OntologyDeltaReport` and
ranked it first.

Treat the *ordering* as a weak hint and the *number* as noise. The CLI prints
this warning in its own output rather than burying it here, because the moment
you need to know is the moment you are reading a score. Build with
`--features embed-onnx` and supply `model.onnx` + `tokenizer.json` for
similarity that means something.

---

## Embedded Studio Web Workbench

Launch the studio with `physis-core studio --port 3000`:

- **Classify Workbench**: Live multi-cell classification, raw vs quality-penalized score comparisons, nearest entry details, and one-click feedback buttons (`✓ Success` / `✕ Failure`).
- **Semiotic Heatmap**: Interactive visual grid of the 5 domains × 14 modes with dynamic axis discovery and cell density mapping.
- **Ontology Editor**: Search, create, and modify domain entries, units, synonyms, and sub-domain facets with instantaneous re-indexing.
- **Corpus & Coherence Graph**: Browse labeled nodes, examine confidence links, and trigger Dream cycles over dissenting paths.
- **Gap Discovery Studio**: Run clustering over unmapped document collections and promote discovered domain clusters into first-class ontology nodes with a single click.
- **Quality & Feedback Matrix**: View active penalties, inspect failure records, and apply boosts to undo historical penalties.

### Environment

| Variable | Default | Effect |
| --- | --- | --- |
| `PHYSIS_CORE_DIR` | `$HOME/.physis-core` | State directory for `nodes.json`, `quality.json` and `custom_ontology.json`. The studio and the CLI both resolve the graph through it, which is why `physis-core scan` and the studio see the same nodes; set it to work on a per-project graph, mount a volume in a container, or keep a test run away from your real graph. |
| `PHYSIS_STUDIO_HOST` | `127.0.0.1` | Bind address. Loopback by default because the ingest and scan routes read arbitrary local paths — only widen it (e.g. `0.0.0.0`) where the port is not publicly reachable. |
| `PHYSIS_EMBEDDER` | auto-detect | `random-projection` selects the deterministic offline embedder: reproducible and coarse, reported as `semantic: false` but *not* as degraded, since it was asked for rather than fallen back to. |

---

## Publishing to Crates.io & Integration

`physis-core` is distributed as a standalone crate and as part of the `physis-pro` industrial suite.

### Publishing Verification

```sh
# Run clippy across all feature sets
cargo clippy --package physis-core --all-targets --all-features -- -D warnings

# Execute test suite
cargo test --package physis-core --all-targets --all-features

# Build documentation locally
cargo doc --package physis-core --all-features --no-deps --open

# Dry-run publish
cargo publish --package physis-core --dry-run
```

---

---

## Practical Examples

### Build a Quality Prediction Agent

```rust
use physis_core::{
    PhysisCore, RandomProjectionEmbedder, VectorEmbed,
    quality::QualityTracker,
    classify::{classify_text, ClassifierConfig},
    ontology::OntologyLoader,
    config::PhysisConfig,
};

fn main() -> anyhow::Result<()> {
    let embedder = RandomProjectionEmbedder::new(64);
    let ontology = OntologyLoader::load_all(&PhysisConfig::default());
    let mut core = PhysisCore::new();
    core.set_embedder_id("random-projection");

    // Load quality tracker (persisted penalties)
    let mut quality = QualityTracker::load("~/.physis-core/quality.json")?;

    // Ingest a production defect report
    let report = "Surface finish degradation on turned parts — chatter marks";
    let classification = classify_text(report, &ontology, &embedder, &ClassifierConfig::default());

    println!("Top cell: {}×{} ({:.2})", 
        classification.top_cell.domain, 
        classification.top_cell.mode, 
        classification.top_cell.score);

    // Apply quality penalty for this cell (learned from past failures)
    let penalty = quality.penalty_for_cell(&classification.top_cell.domain, &classification.top_cell.mode);
    println!("Learned penalty: {:.2}", penalty);

    // Register the defect with asserted failure
    let node_id = core.register_node_from_text(report, &embedder)?;
    core.assert_coherence(&node_id, -1.0);

    // Save updated quality tracker
    quality.save("~/.physis-core/quality.json")?;

    Ok(())
}
```

### Competing Hypotheses for Root Cause Analysis

```rust
use physis_core::{
    PhysisCore, Hypothesis, HypothesisStatus, Evidence, EvidencePolarity,
    RandomProjectionEmbedder, VectorEmbed,
};

fn main() {
    let embedder = RandomProjectionEmbedder::new(128);
    let mut core = PhysisCore::new();

    // Hypothesis A: Thermal issue
    let emb_a = embedder.embed("Nozzle temperature instability causes layer adhesion failure");
    let mut hyp_a = Hypothesis::new("Thermal instability", emb_a);
    hyp_a.assumptions.push("Thermistor reads true melt temperature".to_string());
    let id_a = core.register_hypothesis(hyp_a);

    // Hypothesis B: Material issue  
    let emb_b = embedder.embed("Wet filament causes steam bubbles and poor layer bonding");
    let mut hyp_b = Hypothesis::new("Moisture contamination", emb_b);
    hyp_b.assumptions.push("Filament stored in sealed container with desiccant".to_string());
    let id_b = core.register_hypothesis(hyp_b);

    // Evidence supporting A
    core.attach_evidence(&id_a, Evidence {
        source: "thermal_camera".to_string(),
        polarity: EvidencePolarity::Supports,
        confidence: 0.92,
        claim: "Nozzle temp oscillates ±8°C during print".to_string(),
        observed_at: Some(chrono::Utc::now()),
        embedding: vec![],
        context: vec!["material: PLA".to_string(), "nozzle: 0.4mm".to_string()],
    });

    // Evidence refuting B
    core.attach_evidence(&id_b, Evidence {
        source: "humidity_sensor".to_string(),
        polarity: EvidencePolarity::Refutes,
        confidence: 0.88,
        claim: "Filament chamber RH 12% — well below absorption threshold".to_string(),
        observed_at: Some(chrono::Utc::now()),
        embedding: vec![],
        context: vec!["chamber: dry_box".to_string()],
    });

    // Update fitness scores
    if let Some(h) = core.hypotheses.get_mut(&id_a) {
        h.status = HypothesisStatus::Supported;
        h.fitness = 0.91;
    }
    if let Some(h) = core.hypotheses.get_mut(&id_b) {
        h.status = HypothesisStatus::Refuted;
        h.fitness = 0.22;
    }

    // Best hypothesis wins
    let best = core.hypotheses.values().max_by(|a,b| a.fitness.partial_cmp(&b.fitness).unwrap());
    println!("Root cause: {} (fitness: {:.2})", best.unwrap().label, best.unwrap().fitness);
}
```

### Token-Budgeted RAG for LLM Context

```rust
use physis_core::{
    RagCorpus, RagChunk, TokenFixedRetriever, RandomProjectionEmbedder, VectorEmbed,
};

fn main() {
    let embedder = RandomProjectionEmbedder::new(64);
    let mut corpus = RagCorpus::new();

    // Ingest technical docs
    corpus.add_chunk(RagChunk::new(
        "sof-001", 
        "SOP-204: Spindle bearing replacement. Torque to 45Nm. Run-in 30min at 500RPM.",
        embedder.embed("spindle bearing replacement torque run-in")
    ));
    corpus.add_chunk(RagChunk::new(
        "sof-002",
        "SOP-312: Coolant filter change. Interval 500h. Use FN-7 filter only.",
        embedder.embed("coolant filter change interval")
    ));
    corpus.add_chunk(RagChunk::new(
        "sof-003",
        "SOP-108: Emergency stop reset. Verify guard closure. Press blue reset 3s.",
        embedder.embed("emergency stop reset guard")
    ));

    let retriever = TokenFixedRetriever::new();
    let query = embedder.embed("bearing torque procedure");

    // Hard budget: 200 tokens max, diversity threshold 0.7
    let result = retriever.retrieve_bounded(&corpus, &query, 200, 0.70);
    
    println!("Retrieved {} chunks, {} tokens", result.chunks.len(), result.total_tokens);
    for chunk in result.chunks {
        println!("  [{}] {}", chunk.id, chunk.text);
    }
}
```

### Ontology Gap Discovery — Find Missing Domains

```rust
use physis_core::{
    discover, DiscoveryConfig, RandomProjectionEmbedder, OntologyLoader, PhysisConfig,
};

fn main() {
    let embedder = RandomProjectionEmbedder::new(64);
    let ontology = OntologyLoader::load_all(&PhysisConfig::default());

    // Unclassified maintenance logs from a new machine type
    let logs = vec![
        "Laser power drift during micro-welding cycle".to_string(),
        "Beam focus position variance exceeds 15 microns".to_string(),
        "Argon shield gas flow instability at 12 L/min".to_string(),
        "Galvo scanner hysteresis on tight radius corners".to_string(),
    ];

    let config = DiscoveryConfig {
        coverage_threshold: 0.75,   // Flag texts below 0.75 similarity
        min_cluster_size: 2,        // Need at least 2 similar texts
        max_clusters: 8,
    };

    let report = discover(&logs, &ontology, &embedder, &config);

    println!("Gap report: {} texts analyzed, {} clusters found", 
        report.analyzed, report.proposed_domains.len());
    
    for (i, domain) in report.proposed_domains.iter().enumerate() {
        println!("  Proposal {}: {}×{} — {}", i+1, domain.domain, domain.mode, domain.rationale);
        println!("    Exemplars: {}", domain.exemplars.join(", "));
    }

    // Promote to ontology (serializes JSON for config/)
    println!("{}", serde_json::to_string_pretty(&report.proposed_domains)?);
}
```

### Contradiction Tracking — Non-Destructive Conflict Resolution

```rust
use physis_core::{
    PhysisCore, Contradiction, ContradictionParty, ResolutionStatus,
};

fn main() {
    let mut core = PhysisCore::new();

    // Sensor A says pressure nominal
    let claim_a = ContradictionParty {
        source: "pressure_transducer_primary".to_string(),
        claim: "System pressure 6.2 bar — within spec".to_string(),
        confidence: 0.91,
        context: vec!["location: manifold".to_string()],
    };

    // Sensor B says cavitation
    let claim_b = ContradictionParty {
        source: "acoustic_emission_sensor".to_string(),
        claim: "Cavitation signatures detected at impeller".to_string(),
        confidence: 0.96,
        context: vec!["location: pump_impeller".to_string()],
    };

    // Register explicit contradiction (both preserved)
    let conflict = Contradiction::new(claim_a, claim_b, 0.93);
    let conflict_id = core.register_contradiction(conflict);

    // Later: resolve with contextual grounding — dissent NOT deleted
    core.resolve_contradiction(
        &conflict_id,
        ResolutionStatus::ResolvedPreferredB,
        "Primary transducer upstream of clogged suction filter; cavitation confirmed by borescope",
    );

    // Audit trail shows full history
    let events = core.epistemic_audit.query(&conflict_id);
    for e in events {
        println!("[{}] {} — {}", e.timestamp, e.event_type, e.note);
    }
}
```

### CLI Workflows

```bash
# Classify a technical issue
physis-core classify "Coolant pump seal leaking at 200h interval"

# Search ontology for relevant entries
physis-core ontology --search "seal"

# Filter by facets (machine process domain)
physis-core facet --kind machine --lifecycle OPERATE

# Ingest a vault of engineering notes
physis-core scan ~/engineering-notes

# Search coherence graph for similar issues
physis-core search "pump seal" --limit 10

# Assert a failure verdict (reinforcement learning signal)
physis-core assert "Coolant pump seal leak" failure

# Run dream cycle to generate countermeasures
physis-core dream

# Discover ontology gaps in unclassified logs
physis-core discover ~/unclassified-logs --min-cluster 3

# Launch embedded studio UI
physis-core studio --port 3000
```

### Embedded Studio Web Workbench

```bash
# Start the studio (feature: studio, enabled by default)
physis-core studio --port 3000 --host 127.0.0.1

# Open http://127.0.0.1:3000 for:
# - Classify Workbench: live multi-cell classification with quality penalties
# - Semiotic Heatmap: interactive 5×14 grid with density mapping
# - Ontology Editor: create/modify domain entries with instant re-indexing
# - Corpus & Coherence Graph: browse nodes, examine confidence links
# - Gap Discovery Studio: cluster unmapped docs → promote to ontology
# - Quality Matrix: view penalties, inspect failures, apply boosts
```

### Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `PHYSIS_CORE_DIR` | `$HOME/.physis-core` | State directory (`nodes.json`, `quality.json`, `custom_ontology.json`) |
| `PHYSIS_STUDIO_HOST` | `127.0.0.1` | Bind address (loopback by default — scan routes read local paths) |
| `PHYSIS_EMBEDDER` | auto-detect | `random-projection` for deterministic offline mode |

---

## License

`physis-core` is dual-licensed under the **Apache License, Version 2.0** ([LICENSE](LICENSE)).

## Structural reasoning (experimental)

Symbolic homomorphism engine (`src/transform.rs`): exact-token variable binding,
six ops (Generalize/Substitute/Invert/Compose/Project/Analogize), 0/1 coherence
gates per application. Design notes: [docs/TRANSFORM_REASONING.md](../docs/TRANSFORM_REASONING.md).
Pinned by 4 lib tests (`reciprocal_resource_exchange_abstraction_transfers`,
`substitution_preserves_invariants`, `absent_relation_yields_incoherence`,
`invalid_analogy_rejected`) — run `cargo test -p physis-core transform::`.
Benchmarks (owner: transform track): `tests/transform_bench.rs`.


---

## Agent skill

An agent skill that uses this engine to make an assistant verify its claims
before making them — four failure modes, each with a mechanical check rather
than an instruction to be careful: <https://github.com/ilPez00/physis-skill>

```sh
curl -fsSL https://raw.githubusercontent.com/ilPez00/physis-skill/main/install.sh | bash
```

## Support

Donations: <https://praxisweb.xyz/me>
