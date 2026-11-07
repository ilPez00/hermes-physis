# hermes-physis

https://github.com/ilPez00/hermes-physis · [install](#install) · [benchmark](#measured)

Evidence-backed code context for Hermes Agent, with both engines bundled and
working offline. No API key, no network call, no Python dependency beyond the
standard library.

The plugin answers four questions a model normally guesses at:

1. **What is in this repo?** - `physis_observe` indexes it into a coherence graph.
2. **What is relevant to this task?** - `physis_context` packs the best nodes
   into a fixed token budget.
3. **Where is it?** - `physis_query` returns scored file/snippet matches.
4. **Did the change move the way it was asked to?** - `physis_diff` runs the
   engine's `direction` check: before/after plus the null of doing nothing.

Plus **replant**: record a verified tool recipe once, then re-execute the real
actions and checks on later identical requests. The model is not called again;
cached successes are never returned. `replant_demo` proves this in one call.

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

## 60-second demo

```
physis_engine_status()                     # engines + store + node count
physis_grid()                              # adaptive grid: 8x20, all 160 cells populated
physis_ingest(source="fs", path="/repo")   # append file observations
physis_observed(limit=10)                  # verify what entered the log
physis_observe(path="/path/to/repo")       # index the repo
physis_query(question="where is auth")     # scored matches
physis_context(task="add token rotation", budget=600)   # code, auto-indexed
physis_context(task="what does the runbook say", mode="docs")
physis_chain(task="coverage gaps in auth", budget=1200)
physis_classify(file="src/auth.py")        # adaptive grid cells
replant_demo()                             # record once, replay twice, 0 provider calls
```

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
| `replant_run` | `run` | replay an exact request, or return miss |
| `replant_record` | `record` | execute + check a recipe, then persist it |
| `replant_stats` | `stats` | records, replays, misses, blocks, replay share |
| `replant_show` | `show` | inspect a learned record |
| `replant_demo` | `record` + `run` + `stats` | zero-config proof, bundled counter fixture |

## Configuration

`plugins.entries.physis.settings`:

| Key | Default | Meaning |
|---|---|---|
| `engine_bin` | `""` | override engine path(s); empty = bundled |
| `auto_observe` | `false` | after write/patch, run bounded `watch --source fs` on the edited folder |
| `budget` | `800` | default `physis_context` token budget (100-4000) |
| `timeout` | `20` | per-subprocess timeout (seconds) |

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
replays with `provider_calls=0`; 47 plugin tests and 295 engine tests cover
the paths.

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
- The index is **global** (`~/.physis`, or `PHYSIS_CORE_DIR`). Scanning two
  repos puts both in one graph; queries can surface nodes from either.
- Auto-indexing is bounded: `physis_context` refuses to scan a corpus over
  ~1500 files and the docs compiler refuses over ~5000; pass a narrower path.
- `physis_why` is **evidence retrieval**, not a causal-chain generator. If the
  label is absent, that is missing evidence, not a verdict.
- replant is deliberately narrow: exact request match, registered tools only.
  A miss is the correct answer when the request differs.

## Development

```bash
python3.13 -m pytest          # 50 tests: unit + integration against the bundled binaries
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
- **Everything looks lexical.** The bundled embedder is random-projection; only
  an ONNX-enabled build with real weights changes that.

## Publishing

The repo is release-ready (MIT plugin, Apache-2.0 engines/model, CI, checksums).
To publish as a new GitHub repo:

```bash
scripts/publish.sh   # gh repo create ilPez00/hermes-physis --public --push
```

Review the tree first: `git status`, `bin/SHA256SUMS`, `THIRD_PARTY_NOTICES.md`.

## License

Plugin: MIT. Bundled engines: Apache-2.0 (their LICENSE/NOTICE files are
preserved in `vendor/`). See `THIRD_PARTY_NOTICES.md`.
