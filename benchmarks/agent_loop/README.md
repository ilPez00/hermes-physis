# Agent-in-the-loop benchmark

Answers one question: does the Physis evidence substrate change **verified
outcomes per unit of model cost**, or only the number of tool calls?

It runs the same 20 tasks under three configurations and reports, per
configuration and task category, how many tasks passed their own verification
command, how many tokens that cost, what it spent, how long it took, and how
often Physis tools appear in the transcript.

## Configurations

| Home | Physis plugin | What the agent has |
|---|---|---|
| `home-plain` | disabled | ordinary Hermes tools only; no skill, no hooks |
| `home-skill` | enabled, defaults | the skill's decision policy; the agent decides when to call |
| `home-auto` | enabled, `pre_context` + `auto_observe` | a fresh evidence packet injected before reasoning, graph follows edits |

Each configuration is a temporary `HERMES_HOME` derived from the operator's
`config.yaml`, with `auth.json` **symlinked**, never copied. Profiles are not
used: this host's gateway serves one profile, so `-p <name>` cannot run a second
home.

Before every plugin-enabled run the harness executes the plugin's own
`physis_observe` on the fixture repository, so the graph the skill tells the
agent to build already exists. `home-plain` gets no index - it has no plugin.
`--no-pre-index` disables this, which is the honest "cold start" variant.

## Task categories

- **fresh (10)** — a change that needs reasoning against code the agent has not
  read. Success = the task's own check command passes.
- **grep (6)** — a one-lookup edit where ordinary search suffices. This is the
  negative control: if auto-context helps here, or costs much more, the
  substrate is a tax rather than an advantage.
- **reuse-probe (4)** — prompts phrased as repeated verified work. No recipe is
  pre-approved, so the correct behaviour is to check
  `physis_recipe_lookup`, find nothing tracked, and reason anyway. The metric is
  whether the agent looks before re-deriving, not whether it replays.

## Run

```bash
python3 benchmarks/agent_loop/run_benchmark.py --dry-run          # no model calls
python3 benchmarks/agent_loop/run_benchmark.py --only fresh-01,grep-01
python3 benchmarks/agent_loop/run_benchmark.py --configs plain,skill,auto
```

Every run is a real model call: 20 tasks x 3 configs = 60 runs. Read the
`--usage-file` totals before running the full set. Results append to
`benchmarks/agent_loop/results.jsonl`; the aggregate table prints at the end.

## What this does not prove

- It does not isolate the skill from the plugin: `plain` removes both, so
  `skill` vs `plain` is the skill-plus-tools effect and `auto` vs `skill` is the
  automatic-injection effect. That is the intended comparison, but it is not a
  factorial design.
- Pre-indexing is done by the harness, not the agent. A cold start
  (`--no-pre-index`) is the more conservative reading of `auto`.
- `physis` in the table counts tool-name mentions in the run's transcript, not
  tool calls. It is a proxy for "did the agent use the substrate".
- The fixtures are small and deterministic by design. They measure the loop, not
  repository scale; a large monorepo may behave differently.
- A passing check is necessary, not sufficient: it does not measure whether the
  agent took a wrong path that happened to satisfy the assertion.
- One-shot mode is single-turn. Multi-turn sessions, where `auto_observe` has
  had edits to index and `pre_context` can inject on later turns, are not
  measured here.

