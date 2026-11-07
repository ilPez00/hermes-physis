# hermes-physis

Evidence-backed code context for Hermes Agent, with both engines bundled and
working offline. No API key, no network call, no Python dependency beyond the
standard library.

The plugin answers four questions a model normally guesses at:

1. **What is in this repo?** - `physis_observe` indexes it into a coherence graph.
2. **What is relevant to this task?** - `physis_context` packs the best nodes
   into a fixed token budget.
3. **Where is it?** - `physis_query` returns scored file/snippet matches.
4. **Did this change alter the file's character?** - `physis_diff` compares the
   semiotic classification of two versions.

Plus **replant**: record a verified tool recipe once, then re-execute the real
actions and checks on later identical requests. The model is not called again;
cached successes are never returned. `replant_demo` proves this in one call.

## What is bundled

| Engine | Version | License | Role |
|---|---:|---|---|
| `bin/physis-core` | 0.1.26 | Apache-2.0 | scan, search, node-search, classify, snapshot, ontology |
| `bin/replant` | 0.3.0 | Apache-2.0 | verified tool-recipe replay |
| `vendor/physis-core` | 0.1.26 | Apache-2.0 | full source snapshot used to build the binary |
| `vendor/physis-replant` | 0.3.0 | Apache-2.0 | full source snapshot used to build the binary |

The binaries are Linux x86_64 and are used by default. `PHYSIS_BIN` or the
`engine_bin` setting overrides discovery; a comma-separated list is accepted.

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
physis_observe(path="/path/to/repo")       # index the repo
physis_query(question="where is auth")     # scored matches
physis_context(task="add token rotation", budget=600)
physis_classify(file="src/auth.py")        # structural profile
replant_demo()                             # record once, replay twice, 0 provider calls
```

## Tools

### physis-core

| Tool | Engine command | Use it for |
|---|---|---|
| `physis_engine_status` | - | versions, capabilities, store path, node count |
| `physis_observe` | `scan <dir>` | index a repo (run first) |
| `physis_snapshot` | `snapshot` | graph health: nodes, coherence index, high/mid/low |
| `physis_context` | `node-search` | fixed-token packet for a task |
| `physis_query` | `search` | similarity search over indexed nodes |
| `physis_why` | `node-search` | evidence/coherence around a named object |
| `physis_classify` | `classify` | 5x14 semiotic grid cells for text or a file |
| `physis_diff` | `classify` x2 | structural profile delta between two files |
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

## Honest limits

- The bundled physis-core uses a **random-projection embedder**. Similarity is
  lexical, not semantic; the engine prints that warning on every search. Set
  `PHYSIS_MODEL_DIR` to real weights only if you have an ONNX-enabled build.
- The index is **global** (`~/.physis`, or `PHYSIS_CORE_DIR`). Scanning two
  repos puts both in one graph; queries can surface nodes from either.
- `physis_why` is **evidence retrieval**, not a causal-chain generator. If the
  label is absent, that is missing evidence, not a verdict.
- replant is deliberately narrow: exact request match, registered tools only.
  A miss is the correct answer when the request differs.

## Development

```bash
python3.13 -m pytest          # 28 tests: unit + integration against the bundled binaries
scripts/build-engines.sh      # rebuild both binaries from vendor/ (needs Rust 1.89+)
```

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

## License

Plugin: MIT. Bundled engines: Apache-2.0 (their LICENSE/NOTICE files are
preserved in `vendor/`). See `THIRD_PARTY_NOTICES.md`.
