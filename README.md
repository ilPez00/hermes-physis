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
- `physis_engine_status`
- `physis_observe`
- `physis_context`
- `physis_query`
- `physis_why`
- `physis_diff`
- `physis_command` (run any advertised command)
- `replant_run`, `replant_stats`, `replant_demo`

## Notes
- The plugin probes the engine's `--help` output and disables unsupported commands.
- `auto_observe` defaults to false to keep edits fast.
- No Python dependencies beyond the standard library.
