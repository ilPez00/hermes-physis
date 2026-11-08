# hermes-physis

https://github.com/ilPez00/hermes-physis · [install](#install) · [benchmark](#measured)

Evidence-backed execution memory for Hermes Agent, with both engines bundled and
working offline. No API key, no network call, no Python dependency beyond the
standard library.

Physis answers four questions a model normally guesses at, and then remembers
the answer as a verified procedure:

1. **What is in this repo?** - `physis_observe` indexes it into a coherence graph.
2. **What is relevant to this task?** - `physis_context` packs the best nodes
   into a fixed token budget.
3. **Where is it?** - `physis_query` returns scored file/snippet matches.
4. **Did the change move the way it was asked to?** - `physis_diff` runs the
   engine's `direction` check: before/after plus the null of doing nothing.

Plus **replant**: record a verified tool recipe once, then re-execute the real
actions and checks on later identical requests. The model is not called again;
cached successes are never returned. Record `artifacts=` with it and the recipe
is also bound to its inputs: change one and replay is refused instead of
repeated. `replant_demo` proves the loop in one call; `/physis replay <prompt>`
repeats it without a model turn at all.

`physis_chain` runs the engine's whole pass - structure, gaps, drift and the
label-permuted control - when a task needs an overview rather than one query.

## What is bundled

| Engine | Version | License | Role |
|---|---:|---|---|
| `bin/physis-core` | 0.2.0 | Apache-2.0 | scan, search, node-search, context, classify, grid, snapshot |
| `bin/replant` | 0.3.0 | Apache-2.0 | verified tool-recipe replay |
| `vendor/physis-core` | 0.2.0 | Apache-2.0 | full source snapshot used to build the binary |
| `vendor/physis-replant` | 0.3.0 | Apache-2.0 | full source snapshot used to build the binary |

The binaries are Linux x86_64 and are used by default. `PHYSIS_BIN` or the
`engine_bin` setting overrides discovery; a comma-separated list is accepted.
`models/bge-small/` ships the MIT bge-small-en-v1.5 uint8 ONNX export, so
semantic embeddings are on with no download and no network.

## Install

```bash
hermes plugins install /path/to/hermes-physis
hermes plugins enable physis
```

Verify:

```bash
hermes plugins list | grep physis
```

In a session, ask for `physis_engine_status`; it should list two engines and
the graph node count.

## 60-second demo: one task, evidence and a repeat

Task: add token rotation to `auth`. Baseline without the plugin: grep, open three
files, paste ~1200 tokens into context, then guess whether the change helped.

```
# Index once.
physis_observe(path="/path/to/repo")          # 62 files -> coherence graph

# Ask, with a budget. Returns the relevant functions plus the evidence trail.
physis_context(task="add token rotation", budget=300, mode="code")
# -> 3 nodes, 176 tokens (4.5x compression vs. ~800 tokens of raw hits)
#    evidence: physis_why("rotate_token") lists the nodes and coherence score

# Make the change, then check direction against the do-nothing baseline.
physis_diff(before="auth.py@before", after="auth.py@after", ask="shorter, safer")
# -> moved: true, retention 0.7

# Repeat the verified deploy with no model call.
replant_demo()   # record once, replay twice, provider_calls = 0

# Bind the recipe to its inputs, then let the ground move.
replant_record(input="deploy api", artifacts=["requirements.txt"], ...)
replant_run(input="deploy api")      # replay, provider_calls = 0
#   -> edit requirements.txt
replant_run(input="deploy api")
# -> blocked: "task inputs changed since verification", changed: ["requirements.txt"]
```

The plugin is not a collection of tools: it is the path from "what is in this
repo" to "did the change move as asked" to "run that again for free" to "do not
run that again, the world changed".

## Tools

### physis-core

| Tool | Engine command | Use it for |
|---|---|---|
| `physis_engine_status` | - | versions, capabilities, store path, node count |
| `physis_observe` | `scan <dir>` | index a repo (run first) |
| `physis_snapshot` | `snapshot` | graph health: nodes, coherence index, high/mid/low |
| `physis_context` | `context` (docs) / `node-search` (code) | fixed-token packet + compression stats |
| `physis_query` | `search` | similarity search over indexed nodes |
| `physis_why` | `node-search` | evidence/coherence around a named object |
| `physis_classify` | `classify` | adaptive-grid cells for text or a file |
| `physis_grid` | `grid` | axes, shape, populated cells, active embedder |
| `physis_ingest` | `watch` | append observations from fs/git/terminal/agent/process/browser |
| `physis_observed` | `observed` | read the observation log with sources and timestamps |
| `physis_diff` | `direction` | did the change move as asked, versus the null |
| `physis_chain` | `chain` | one pass: structure, gaps, drift, control |
| `physis_command` | any advertised | escape hatch for advanced commands |

### replant

| Tool | Engine command | Use it for |
|---|---|---|
| `replant_run` | `run` | replay an exact request, or return miss/blocked |
| `replant_record` | `record` | execute + check a recipe, then persist it (with optional `artifacts=` fingerprints) |
| `replant_stats` | `stats` | records, replays, misses, blocks, replay share |
| `replant_show` | `show` | inspect a learned record |
| `replant_demo` | `record` + `run` + `stats` | zero-config proof, bundled counter fixture |
| `physis_candidates` | - | workflows learned from verified sessions, awaiting approval |

### Learned workflows (opt-in)

With `auto_learn: true` and `verify_cmd` set, a session that passes its check is
turned into a **candidate**: the request it answered (`pre_llm_call`), the exact
foreground `terminal` commands, the files it edited, and the check that passed.
Candidates are stored under `.replant/candidates/` and nothing replays until an
operator approves one by id. Approval executes the steps and the check through
replant (`provider_calls: 0`), which is what makes them replayable later.

```bash
/physis candidates          # id, request, steps, files, check
/physis approve 32d149d9    # record it: replay is now allowed
/physis replay say hi       # operation: replay, provider_calls: 0
```

The generated registry names only `stepN` / `verify` and always runs them
through `sys.executable` plus a fixed wrapper, so a learned plan can never
choose its own executable.

### Command

`/physis status` (engines + graph), `/physis recipes` (receipt stats),
`/physis candidates` (learned, unapproved work), `/physis replay <prompt>`
(repeat a recorded recipe with no model call), `/physis approve <id>` (record a
candidate). Set `replant_tools` when the registry is not
`<path>/.replant/tools.json`; an approved candidate resolves to
`<path>/.replant/learned/tools.json` automatically.

### Hooks

- `post_tool_call` - with `auto_observe: true`, re-indexes the edited folder;
  with `auto_learn: true`, records replayable steps (errors are skipped).
- `pre_verify` - with `verify_cmd` set, runs that command after code edits; a
  non-zero exit keeps the turn going with the real failure tail, so a
  deterministic check decides whether the work is done. A passing run is what
  creates a candidate.
- `pre_llm_call` - with `auto_learn: true`, remembers the user request so a
  candidate knows which prompt it answers.

## Configuration

`plugins.entries.physis.settings`:

| Key | Default | Meaning |
|---|---|---|
| `engine_bin` | `""` | override engine path(s); empty = bundled |
| `model_dir` | `""` | ONNX embedding model directory; empty = bundled BGE-small |
| `semantic` | `true` | use the semantic embedder; `false` = deterministic hashing (reproducible, not semantic) |
| `auto_observe` | `false` | after write/patch, re-index (`scan`) the edited folder |
| `verify_cmd` | `""` | command run by the `pre_verify` hook after edits; non-zero keeps the turn going |
| `auto_learn` | `false` | after `verify_cmd` passes, save the session as an approvable workflow candidate |
| `replant_tools` | `""` | tools registry for `/physis replay`; empty = `<path>/.replant/tools.json`, or the learned one if present |
| `budget` | `800` | default `physis_context` token budget (100-4000) |
| `timeout` | `20` | per-subprocess timeout (seconds) |

## Environment variables

All settings above can also be driven (or overridden) through environment
variables. These are read by the engine directly and work without Hermes config.

| Variable | Default | Effect |
|---|---|---|
| `PHYSIS_BIN` | (bundled) | override engine path(s); comma-separated list accepted |
| `PHYSIS_MODEL_DIR` | (bundled BGE-small) | ONNX embedding model directory |
| `PHYSIS_MODEL_DIM` | `384` | embedding dimension; must match `PHYSIS_MODEL_DIR` |
| `PHYSIS_SEMANTIC` | `1` | `0`/`false`/`no`/`off` disables the semantic embedder (deterministic hashing instead) |
| `PHYSIS_CACHE_DIR` | (bundled cache) | ontology-embedding cache directory (~3.5 MB, ~0.2 s first call) |
| `PHYSIS_CORE_DIR` | per-repo hash | graph store location; set explicitly for one shared store |
| `PHYSIS_AUTO_EXPAND` | `1` | `0` disables learning a new ontology entry on uncertain classifications |
| `PHYSIS_ONTOLOGY_DIR` / `PHYSIS_ONTOLOGY_FILES` | (bundled) | add axes or entries at runtime |
| `PHYSIS_NO_...` | - | see `--no-expand` flag on `physis_classify` |

### Turning semantics off

Set `semantic: false` in `plugins.entries.physis.settings`, or `PHYSIS_SEMANTIC=0` in
the environment. The engine falls back to deterministic hashing: reproducible and
offline, but not semantic. Useful for reproducible CI or to drop the embedder
dependency entirely.

### Swapping the embedder

1. Place an ONNX embedding model (`model.onnx` + `tokenizer.json`) in a directory.
2. Set `model_dir` to that directory (or `PHYSIS_MODEL_DIR`).
3. Set `PHYSIS_MODEL_DIM` to the model's output dimension (e.g. `768` for BGE-base).
4. Restart the session. `physis_grid` reports the active embedder.

The model must use the same loader as the bundled BGE-small (CLS pooling, uint8
or float). Heavier models trade bundle size and first-call latency for accuracy;
the default keeps the "no download, no network" guarantee.

## Measured

Classification on two fixtures, reproducible with the bundled binary:

```bash
PHYSIS_CORE_DIR=/tmp/physis-bench ./bin/physis-core \
  bench-semiotic benchmarks/semiotic_cases.json
```

| Fixture | Cases | top-1 cell | top-3 cell | domain | mode |
|---|---|---|---|---|---|
| calibration | 55 | 60.0% | 76.4% | 67.3% | 72.7% |
| validation (authored after tuning) | 30 | 26.7% | 56.7% | 63.3% | 46.7% |

With 160 cells the random baselines are 0.6% (top-1), 1.9% (top-3),
12.5% (domain) and 5.0% (mode). The gap between the two fixtures is the
honest limit: the grid is a coarse lens, not an oracle. On unfamiliar phrasing
prefer the top-3 cells plus the `best_domain` / `best_mode` axes, and let
auto-expansion record what the ontology missed.

Context and replay are deterministic and measured separately: 62 Rust files
compress to 176 tokens at a 300-token budget; a recorded replant recipe
replays with `provider_calls=0`; 60 plugin tests and 295 engine tests cover
the paths.

Verified loop, measured on the bundled fixtures (real binaries, temp store):

| Step | Command | Result |
|---|---|---|
| record with `artifacts=["input.txt"]` | `replant_record` | `status: succeeded`, `provider_calls: 0` |
| replay unchanged inputs | `replant_run` | `operation: replay`, `provider_calls: 0` |
| replay after editing `input.txt` | `replant_run` | `blocked: true`, `changed: ["input.txt"]`, engine never invoked |
| per-repo isolation | `physis_observe` on A, then `physis_snapshot` on B | A `nodes: 1`, B `nodes: 0` |
| learn a verified session | `pre_verify` passing with `auto_learn: true` | 1 candidate, 0 replays |
| approve it | `/physis approve <id>` | `status: succeeded`, `provider_calls: 0` |
| replay it | `/physis replay <prompt>` | `operation: replay`, `provider_calls: 0` |

## Honest limits

- The bundled embedder is **semantic** (BGE-small uint8, 384d, CLS pooling) and on by default.
  The ontology-embedding cache ships warm (`models/cache`, ~3.5 MB), so the
  first call is ~0.2 s instead of ~6 s; `PHYSIS_CACHE_DIR` overrides it.
- The grid is **adaptive**: it is whatever the loaded ontologies define. The
  bundle ships 8 domains x 20 modes, all 160 cells populated (896 entries),
  including agentic (GOVERN, DELEGATE, REFLECT, VERIFY, INFER) and
  agent/world packs derived from physis-next's entity kinds.
- **Auto-expansion**: when a classification is a tie (margin < 0.02), the
  engine adds one learned entry to the top domain's INFER cell and persists it
  to `custom_ontology.json`; the entry count grows on the next call.
  `--no-expand` or `PHYSIS_AUTO_EXPAND=0` disables it. Near duplicates
  (Jaccard >= 0.6) are refused, the store is capped at 500 learned entries,
  and texts under 20 chars are ignored.
- Add axes or entries at runtime with `PHYSIS_ONTOLOGY_DIR` /
  `PHYSIS_ONTOLOGY_FILES`; the grid, cache config and classifier pick them up.
- `physis_context` defaults to `mode=code` (the scanned graph). `mode=docs`
  runs the compiler, which now ingests **any text file** (source included),
  skips binaries and build dirs, and chunks documents so a file larger than
  the budget can still contribute.
- The index is **per repository**: the graph store is derived from the resolved
  `path`, so scanning two repos keeps their nodes apart. `PHYSIS_CORE_DIR` still
  overrides it with one explicit store (the pre-1.5 behaviour). Queries can no
  longer surface nodes from an unrelated project.
- Auto-indexing is bounded: `physis_context` refuses to scan a corpus over
  ~1500 files and the docs compiler refuses over ~5000; pass a narrower path.
- Replay freshness is per file: `artifacts=` hashes each recorded input (≤1 MiB
  each) at record time; a changed or missing input blocks replay. It does not
  hash a whole tree, and it does not replace replant's own fingerprint of the
  tool implementation.
- `pre_verify` runs one command, once per turn (`attempt` > 0 is a no-op), with
  the working directory of the first changed file.
- Learning is opt-in and **advisory**: only foreground `terminal` commands
  (≤2000 chars) and the edited files are kept, up to 12 steps and 16 files per
  session, 32 sessions in memory. A candidate is written only when both an
  intent (`pre_llm_call`) and a passing check exist. Nothing learned replays
  until an operator approves it by id; approval re-runs the steps and the check
  against the current tree, so a stale candidate fails at approval, not later.
- `physis_why` is **evidence retrieval**, not a causal-chain generator. If the
  label is absent, that is missing evidence, not a verdict.
- replant is deliberately narrow: exact request match, registered tools only.
  A miss is the correct answer when the request differs.

## Development

```bash
python3.13 -m pytest          # 60 tests: unit + integration against the bundled binaries
scripts/build-engines.sh      # rebuild both binaries from vendor/ (needs Rust 1.89+)
```

Rebuilding from `vendor/` reproduces the shipped binaries byte-for-byte (same
SHA256; see `bin/SHA256SUMS`), so the source snapshot is the one that built the
bundle.

The integration tests are the regression net for the original packaging bug:
unit tests with fake engines passed while the shipped binary advertised a
different command vocabulary. `tests/test_integration.py` runs the real
binaries in an isolated `PHYSIS_CORE_DIR`.

## Layout

```
__init__.py          register(ctx): tools, hook, skill
tools.py             handlers + register_tools(ctx) + post_tool_call hook
schemas.py           tool schemas (descriptions are operating instructions)
engine.py            discovery, capability probing, subprocess execution
bin/                 bundled binaries + SHA256SUMS
vendor/              source snapshots for both engines
examples/            zero-config replant fixture
skills/physis/       the skill Hermes loads with the plugin
tests/               unit + integration
```

## Troubleshooting

- **Installer flags the bundle.** Hermes scans third-party plugins. The two
  Linux binaries are the only opaque files; their hashes are in
  `bin/SHA256SUMS` and they rebuild from the vendored source. Review, then
  re-run with `--force` if your installer asks.
- **`No recalled nodes`.** The index is empty for that store; run
  `physis_observe` first, or check `PHYSIS_CORE_DIR`.
- **`physis_context` says "no corpus documents".** It compiles markdown/text
  documents; point `path` at a directory that contains them.
- **Everything looks lexical.** The bundled embedder is BGE-small-en-v1.5 (ONNX,
  384d, CLS pooling). If you override `model_dir`, point it at a directory
  containing `model.onnx` and `tokenizer.json`; an empty or wrong directory
  falls back to deterministic hashing, which is reproducible but not semantic.

## Publishing

The plugin is published at https://github.com/ilPez00/hermes-physis
(release v1.6.0). It is listed in the Hermes plugin catalog via PR #151275
(pinned to the attested commit 8602dbbccea9e9a80944ce847b3c2fad0d8b3ec0).

To ship a new version:

```bash
scripts/publish.sh   # bumps nothing by itself; review, tag, push
```

Then open a SHA-bump PR against NousResearch/hermes-agent updating
`plugin-catalog/physis.yaml`. Review the tree first: `git status`,
`bin/SHA256SUMS`, `THIRD_PARTY_NOTICES.md`.

## License

Plugin: MIT. Bundled engines: Apache-2.0 (their LICENSE/NOTICE files are
preserved in `vendor/`). See `THIRD_PARTY_NOTICES.md`.
