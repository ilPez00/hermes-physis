# hermes-physis

Evidence-backed code context for Hermes Agent using a lightweight Physis engine
(`physis-core` or `physis-replant`; full `physis` is optional).

## Install
```bash
hermes plugins install /path/to/hermes-physis
hermes plugins enable physis
```

## Configure
Set `PHYSIS_BIN` or `plugins.entries.physis.settings.engine_bin` to your engine binary.

## Tools
- `physis_engine_status` — available engines and their supported commands
- `physis_observe` — scan a directory into the coherence graph (`physis-core scan`)
- `physis_context` — bounded context packet for a task (`physis-core context`)
- `physis_query` — structural search (`physis-core search`)
- `physis_why` — common ground / evidence for an object (`physis-core ground`)
- `physis_diff` — direction check between two files (`physis-core direction`)
- `physis_command` — run any advertised engine command
- `replant_run` — replay a verified recipe or return miss (replant `run`)
- `replant_record` — record a verified recipe (replant `record`)
- `replant_stats` — replay counts and effectiveness (replant `stats`)
- `replant_demo` — record→replay loop demo
- `replant_show` — show a learned record (replant `show`)

## Configure
Set `PHYSIS_BIN` or `plugins.entries.physis.settings.engine_bin` to your engine binary.
The plugin ships pre-built binaries in `bin/` (physis-core, replant) and uses them by default.

## Notes
- The plugin probes the engine's `--help` output and disables unsupported commands.
- `auto_observe` defaults to false to keep edits fast.
- No Python dependencies beyond the standard library.
- `replant_*` tools require a `tools` path (replant tools registry JSON) and optionally a `store` path.
