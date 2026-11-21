#!/usr/bin/env python3.13
"""Real-corpus measurement of gated observer fusion.

Corpus: 24 real physis-next plan docs (engine-visible).
Held-out: 3 real plan docs the engine can NEVER see (observer-only knowledge).
Noise: verbatim excerpts from genuinely unrelated real repos.
Embedder: production bge-small (semantic), via PHYSIS_MODEL_DIR.

Per query: baseline packet (no claims) vs fused packet (relevant claim + 2
noise claims) at the embedder-aware default floor, plus one raw-relevance
probe at floor 0.0. Packet-level checks are distinctive-phrase presence.
"""
import json
import os
import subprocess
import sys

BUDGET = "1200"
ENGINE = "/home/gio/.local/bin/physis-core"
CORPUS = "/tmp/real-fusion/corpus"
import sys as _sys
if _sys.argv[-1] == "--rp":
    ENV = dict(os.environ)  # no PHYSIS_MODEL_DIR: the CLI uses random-projection
    _sys.argv = _sys.argv[:-1]
else:
    ENV = {
        **os.environ,
        "PHYSIS_MODEL_DIR": os.environ.get("PHYSIS_BENCH_MODEL_DIR",
                                           "/home/gio/dev/hermes-physis/models/bge-small"),
        "PHYSIS_MODEL_DIM": "384",
    }

def claim(cid, oid, snippet):
    return {
        "id": cid,
        "source": {"uri": "doc://heldout"},
        "extractor": {"id": "heldout-docs"},
        "payload": {"kind": "retrieval", "data": {"object_id": oid, "score": 0.5, "snippet": snippet}},
    }

NOISE = [
    claim(
        "n1",
        "obj:noise-nas",
        "Read-only inventory, change tracking and evidence tooling for an explicitly "
        "mounted share, using the existing Physis CLI. A separate local index is kept "
        "for each canonical mount path. This is not a UGREEN Skill or a hackathon "
        "submission, and UGREEN runtime compatibility has not been established.",
    ),
    claim(
        "n2",
        "obj:noise-playground",
        "An interactive, public playground for learning how Physis builds a world "
        "model: observations ground facts, identity joins domains, rules derive with "
        "named premises, and truth maintenance keeps the world honest under change.",
    ),
]

QUERIES = [
    {
        "name": "gate-ambient-state (held-out)",
        "query": "why did gate verdicts report confident false negatives",
        "claim": claim(
            "h1",
            "obj:gate-ambient",
            "Gate verdicts depend on ambient state. One false negative was a stale "
            "cached rlib: cargo test -p physis-hud reported E0027/E0063 demanding a "
            "field observed on physis_notebook::Cell::Ref — a field that does not "
            "exist. A false RED from a stale build artifact, not from the code.",
        ),
        "phrase": "Stale cached rlib" if False else "stale cached rlib",
        "phrase_ci": True,
    },
    {
        "name": "symbolic-composition (held-out)",
        "query": "which primitive families are worth adding for autonomous composition",
        "claim": claim(
            "h2",
            "obj:symbolic-composition",
            "What is missing is not a new symbolic AI but three specific closures: "
            "programs that carry dataflow, a bounded planner that assembles programs "
            "from goals, and an executor that actually runs the program it compiled. "
            "Highest-value primitive family: natural-language-in. Second: a small set "
            "of logic glue primitives. Physics: no — zero demand, zero detectors, "
            "zero corpus, and primitive admission is demand-gated by policy.",
        ),
        "phrase": "demand-gated by policy",
    },
    {
        "name": "language-organs (held-out)",
        "query": "what did the language organs first loop accept",
        "claim": claim(
            "h3",
            "obj:language-organs",
            "This first loop is a verified subset, not the final language ambition. "
            "The broader goal is ordinary general-language understanding.",
        ),
        "phrase": "verified subset, not the final language ambition",
    },
    {
        "name": "architectural-convergence (held-out)",
        "query": "how do the kernel and store surfaces converge on shared contracts",
        "claim": claim(
            "h4",
            "obj:arch-convergence",
            "Convergence means shared, testable contracts and honest interop. It does "
            "not mean collapsing crates or claiming every surface is one fully "
            "synchronized database. Observation log sequence, event revision, Git "
            "commit, source content digest, and root namespace remain distinct "
            "evidence until a design proves a safe unification.",
        ),
        "phrase": "remain distinct evidence until a design proves a safe unification",
    },
    {
        "name": "impossibllm-reasoning-ladder (held-out)",
        "query": "how does the cascade decide between reasoning engines",
        "claim": claim(
            "h5",
            "obj:cascade-decide",
            "Cascade::decide walks ordered engines and accepts the first decision "
            "with confidence at or above the accept threshold; otherwise it retains "
            "the best sub-threshold decision above the escalate floor; everything "
            "else is a hard abstain with escalation flagged. Tiers: compiled rules, "
            "embed, causal logit, tiny scorer, transition cache.",
        ),
        "phrase": "hard abstain with escalation flagged",
    },
    {
        "name": "shock-economic-demo (held-out)",
        "query": "what should the shock economic value demo show a maintainer",
        "claim": claim(
            "h6",
            "obj:shock-demo",
            "Make a maintainer see something useful within one minute, complete a "
            "trustworthy change within one session, and give an agent less paid "
            "context per accepted task. The primary audiences are developers working "
            "across unfamiliar repositories and teams paying for repeated agent "
            "investigation, review, and repairs.",
        ),
        "phrase": "less paid context per accepted task",
    },
    {
        "name": "release-closeout (held-out)",
        "query": "how was the 1.2.0 release certified for exact bytes",
        "claim": claim(
            "h7",
            "obj:release-closeout",
            "Build physis and physis-context 1.2.0 from a pinned commit in a clean "
            "detached worktree using the production verification pubkey, certify the "
            "exact shipped bytes with supplied-mode acceptance plus value-check, and "
            "publish the immutable v1.2.0 tag.",
        ),
        "phrase": "certify the exact shipped bytes",
    },
    {
        "name": "physis-cc (held-out)",
        "query": "what does the executable intent compiler promise",
        "claim": claim(
            "h8",
            "obj:physis-cc",
            "Turn bounded natural-language instructions into inspectable, validated, "
            "replayable programs grounded in the local observed world. Keep "
            "interpretation, execution, and evidence distinct. Native programs must "
            "state assumptions and gaps; no artifact may grant itself permissions.",
        ),
        "phrase": "no artifact may grant itself permissions",
    },
    {
        "name": "gui-navigation (held-out)",
        "query": "how should the physis GUI navigation shell behave",
        "claim": claim(
            "h9",
            "obj:gui-navigation",
            "Turn physis-gui into the computer-navigation surface: it opens on a "
            "graph of the observed world, and panels switch between graph, search, "
            "context, relations, notebook and editor. The Notebook and Editor tabs "
            "already exist in the GUI's mirror of the HUD tab set but are "
            "unimplemented in implemented().",
        ),
        "phrase": "opens on a graph of the observed world",
    },
    {
        "name": "impossibllm-rescope (held-out)",
        "query": "what landed for impossibLLM and what stays operator-gated",
        "claim": claim(
            "h10",
            "obj:impossibllm-rescope",
            "L0 to L3 landed as four commits on the impossibllm-rescope branch and "
            "was fast-forward merged to main. Publication, tagging and pricing "
            "remain operator-gated — the charter authorizes none of them by itself. "
            "L4, the measurement arms, is a pre-registered protocol, not a result.",
        ),
        "phrase": "publication, tagging and pricing remain operator-gated",
    },
    {
        "name": "invariant-coverage (held-out)",
        "query": "what does the physis invariants report currently cover",
        "claim": claim(
            "h11",
            "obj:invariant-coverage",
            "Turn Physis invariants from a two-item, partly UNKNOWN report into an "
            "executable, expanding coverage set. The report currently shows "
            "append-only-history PASS and deterministic-replay UNKNOWN; "
            "evaluate_invariants accepts an optional replay_deterministic flag.",
        ),
        "phrase": "executable, expanding coverage set",
    },
    {
        "name": "defect-remediation (held-out)",
        "query": "what was the first defect remediation priority",
        "claim": claim(
            "h12",
            "obj:defect-remediation",
            "Order is ROI, not severity: the first item decides whether anything "
            "else can be sold. D0: build the deciding eval before touching "
            "retrieval — one script, one number, recall at cost against grep plus "
            "read, over twenty tasks split between prose and code.",
        ),
        "phrase": "recall at cost against grep plus read",
    },
    {
        "name": "economic-relevance (held-out)",
        "query": "what must the engineering prove for the commercial offer to be honest",
        "claim": claim(
            "h13",
            "obj:economic-relevance",
            "The commercial plan is a 190 euro structural review of one repository "
            "in one day, with no licence until the install path verifies. The open "
            "question: what does the engineering have to be true for that offer to "
            "be honest, and what is still false?",
        ),
        "phrase": "for that offer to be",
    },
    {
        "name": "viewspec-lens (held-out)",
        "query": "how does the typed viewspec get validated against the canonical world",
        "claim": claim(
            "h14",
            "obj:viewspec-lens",
            "One typed ViewSpec compiled from the canonical world, validated "
            "against a named gate before any component renders a retraction; "
            "the card is not admissible with a weaker gate.",
        ),
        "phrase": "validated against a named gate",
    },
    {
        "name": "dream-grading (held-out)",
        "query": "when is a dream grade recorded and what basis does it need",
        "claim": claim(
            "h15",
            "obj:dream-grading",
            "A grade is recorded only with a named basis, stored append-only, "
            "and exposed without inventing scores; candidates without a basis "
            "stay ungraded.",
        ),
        "phrase": "stored append-only",
    },
    {
        "name": "sovereign-sequence (held-out)",
        "query": "what is the first phase of the sovereign execution sequence",
        "claim": claim(
            "h16",
            "obj:sovereign-sequence",
            "Phase 1 is the reversible action engine: an action state dock, a "
            "visual pre-flight preview, and a time-travel undo scrubber, "
            "sequenced by most valuable actions first.",
        ),
        "phrase": "time-travel undo scrubber",
    },
    {
        "name": "praxis-census (held-out)",
        "query": "how many praxis files import the physis pro client",
        "claim": claim(
            "h17",
            "obj:praxis-census",
            "The census measured 17 files importing physisProClient, counted "
            "by grep with no edits made, as the removal track's baseline "
            "before any file is touched.",
        ),
        "phrase": "17 files importing",
    },
    {
        "name": "full-review-verified (held-out)",
        "query": "what did the comprehensive physis review verify on main",
        "claim": claim(
            "h18",
            "obj:full-review-verified",
            "The review reran the gates on main the same day: 340 tests "
            "passed across physis-cli and physis-mcp, and every number in the "
            "review is either measured that day or cited to its receipt.",
        ),
        "phrase": "either measured that day or cited",
    },
    {
        "name": "gui-pulse-bar (held-out)",
        "query": "what does the pulse bar in the gui overhaul display",
        "claim": claim(
            "h19",
            "obj:gui-pulse-bar",
            "The pulse bar is one organism across the window: vitals on the "
            "left, the command box centered, the agent chip and help on the "
            "right, with live ingest reactions flashing glyph ticks.",
        ),
        "phrase": "vitals on the",
    },
    {
        "name": "blast-radius-timing (held-out)",
        "query": "how long did the ansible blast radius observation take",
        "claim": claim(
            "h20",
            "obj:blast-radius-timing",
            "The ansible observation took 2.0 seconds for 5,824 files, "
            "26,350 entities and 35,932 relations, cloned at depth one and "
            "measured by command, not grep.",
        ),
        "phrase": "26,350 entities",
    },
    {
        "name": "semantic-lens-exclusions (held-out)",
        "query": "what does the semantic lens report about what it excluded",
        "claim": claim(
            "h21",
            "obj:semantic-lens-exclusions",
            "The lens reports what it excluded and why, tracks how the "
            "concept changes across contexts, and states what is proved, "
            "refuted or unknown, with no relation claiming more evidence "
            "than it has.",
        ),
        "phrase": "excluded and why",
    },
    {
        "name": "workbench-receipts (held-out)",
        "query": "what does the workbench shell attach to every rendered fact",
        "claim": claim(
            "h22",
            "obj:workbench-receipts",
            "Every rendered fact in the workbench carries a receipt naming "
            "the tool, its arguments, and the seq or revision behind it, "
            "with lenses sharing one selection and one as-of cursor.",
        ),
        "phrase": "the seq or revision behind it",
    },
    {
        "name": "durable-tms (held-out)",
        "query": "what does durable truth maintenance analyse retractions with",
        "claim": claim(
            "h23",
            "obj:durable-tms",
            "Durable truth maintenance v1 is justification-aware retraction "
            "analysis: a retraction is analysed against the justifications "
            "that support a belief, not just flipped off, and it is operator "
            "milestone 3 of the 2026-11-17 directive.",
        ),
        "phrase": "justification-aware retraction",
    },
    {
        "name": "rau-semantics (held-out)",
        "query": "what finding drove the relation admission unification",
        "claim": claim(
            "h24",
            "obj:rau-semantics",
            "The finding: Physis does not yet apply the same semantic rules "
            "to every source entering its world model — five direct-insertion "
            "paths bypass the admission gate that observed input passes "
            "through.",
        ),
        "phrase": "five direct-insertion paths",
    },
    {
        "name": "temporal-defect (held-out)",
        "query": "what did the temporal entailment defect admit",
        "claim": claim(
            "h25",
            "obj:temporal-defect",
            "The OWL transitive-closure block admitted every entailed edge as "
            "TemporalValidity::permanent, and stamp_temporal_validity then "
            "backfilled that permanence over derived edges that should have "
            "inherited their own bounds.",
        ),
        "phrase": "backfilled that permanence",
    },
    {
        "name": "paired-bench-metric (held-out)",
        "query": "what does the paired dev benchmark measure per agent-hour",
        "claim": claim(
            "h26",
            "obj:paired-bench-metric",
            "The paired dev benchmark compares Physis-assisted development "
            "with conventional development on identical tasks and models, "
            "measuring correct fixes per agent-hour, token usage, time, "
            "regressions and interventions.",
        ),
        "phrase": "correct fixes per agent-hour",
    },
    {
        "name": "playground-phase1 (held-out)",
        "query": "what landed in phase one of the world model playground",
        "claim": claim(
            "h27",
            "obj:playground-phase1",
            "Phase 1 of the world model playground landed the Physis "
            "branding, the canonical fixture with an official physis-world "
            "catalogue entry, and the base import adapted into the "
            "playground repo.",
        ),
        "phrase": "canonical fixture with an official",
    },
    {
        "name": "hackathon-dating (held-out)",
        "query": "why does the hackathon plan schedule engine work against real dates",
        "claim": claim(
            "h28",
            "obj:hackathon-dating",
            "The hackathon milestones plan treats the operator's external "
            "hackathon pipeline as engine milestones, so physis-next work is "
            "scheduled against real dates instead of open-ended someday.",
        ),
        "phrase": "instead of open-ended someday",
    },
    {
        "name": "spaceship-matrix (held-out)",
        "query": "what did the spaceship reverse-engineering produce",
        "claim": claim(
            "h29",
            "obj:spaceship-matrix",
            "The spaceship reverse-engineering produced an integration "
            "matrix for interactive world steering, working from a "
            "read-only MIT-licensed clone with a citation request, not a "
            "fork.",
        ),
        "phrase": "produced an integration",
    },
    {
        "name": "steering-ir-rule (held-out)",
        "query": "what rule does the steering IR design honor",
        "claim": claim(
            "h30",
            "obj:steering-ir-rule",
            "The SteeringIR and counterfactual branches design honors one "
            "rule: no new world model, no second ontology, no second "
            "persistence layer — everything is a surface over the existing "
            "fold, store and invariants.",
        ),
        "phrase": "no second ontology",
    },
    {
        "name": "quarry-migration (control, corpus-covered)",
        "query": "what is the quarry migration plan status",
        "claim": None,
        "phrase": "sequencing authority for the",
    },
]


RESERVE = sys.argv[1] if len(sys.argv) > 1 else "auto"
FLOOR = sys.argv[2] if len(sys.argv) > 2 and sys.argv[2] != "--rp" else None

def run_context(query, claims, floor=None):
    argv = [ENGINE, "context", "--corpus", CORPUS, "--query", query, "--budget", BUDGET, "--json"]
    if claims is not None:
        path = "/tmp/real-fusion/claims-live.json"
        with open(path, "w") as f:
            json.dump(claims, f)
        argv += ["--observer-claims", path, "--observer-reserve", RESERVE]
        if floor is not None:
            argv += ["--observer-floor", str(floor)]
    out = subprocess.run(argv, capture_output=True, text=True, env=ENV)
    if out.returncode != 0:
        raise RuntimeError(out.stderr)
    return json.loads(out.stdout)


rows = []
for q in QUERIES:
    base = run_context(q["query"], None)
    base_hit = q["phrase"].lower() in base["physis_context"].lower()

    claims = ([q["claim"]] if q["claim"] else []) + NOISE
    fused = run_context(q["query"], claims, floor=FLOOR)
    fused_hit = q["phrase"].lower() in fused["physis_context"].lower()
    noise_in = any(
        o["decision"] == "AcceptCandidate" and o["object_id"].startswith("obj:noise")
        for o in fused["observer_gated"]
    )
    noise_text_in = any(
        w in fused["physis_context"].lower()
        for w in ("ugreen runtime compatibility", "public playground for learning")
    )

    probe = run_context(q["query"], claims, floor=0.0)
    relevances = {o["object_id"]: round(o["relevance"], 4) for o in probe["observer_gated"]}

    rows.append(
        {
            "query": q["name"],
            "baseline_phrase_present": base_hit,
            "fused_phrase_present": fused_hit,
            "observer_selected": fused["observer_selected"],
            "noise_admitted": noise_in,
            "noise_text_in_packet": noise_text_in,
            "relevances": relevances,
            "physis_tokens_base": base["physis_tokens"],
            "physis_tokens_fused": fused["physis_tokens"],
        }
    )

print(json.dumps(rows, indent=2))
with open("/tmp/real-fusion/results.json", "w") as f:
    json.dump(rows, f, indent=2)
