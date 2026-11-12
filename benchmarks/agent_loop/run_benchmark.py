#!/usr/bin/env python3
"""Agent-in-the-loop benchmark for the Physis plugin.

Runs the same task set under three Hermes configurations and reports verified
outcomes per unit of model cost:

  plain  - the physis plugin disabled (no skill, no tools, no hooks)
  skill  - plugin enabled with defaults: the skill is available, the agent decides
  auto   - plugin enabled with pre_context + auto_observe: the evidence packet is
           injected before the model reasons and the graph follows edits

The metric that matters is not how often Physis is called. It is how much
verified work completes per token, and whether the negative controls (tasks where
ordinary search suffices) get slower or more expensive.

Configuration isolation uses HERMES_HOME, not Hermes profiles: this host's
gateway serves one profile, so `-p <profile>` cannot run a second home. Each
config gets a temporary home derived from the current config.yaml, with
auth.json symlinked (never copied) so no credential is duplicated.

Usage:
    python3 benchmarks/agent_loop/run_benchmark.py --dry-run
    python3 benchmarks/agent_loop/run_benchmark.py --configs plain,skill,auto
    python3 benchmarks/agent_loop/run_benchmark.py --only fresh-01,grep-01

Spend: every task x config is a real model run. 20 tasks x 3 configs = 60 runs.
Read the --usage-file totals before trusting a large run.
"""

from __future__ import annotations

import argparse
import copy
import json
import os
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

import yaml

HERE = Path(__file__).resolve().parent
DEFAULT_TASKS = HERE / "tasks.jsonl"
PLUGIN_DIR = HERE.parents[1]

CONFIGS: dict[str, dict[str, Any]] = {
    "plain": {"enabled": False, "settings": {}},
    "skill": {"enabled": True, "settings": {}},
    "auto": {
        "enabled": True,
        "settings": {
            "pre_context": True,
            "pre_context_budget": 300,
            "pre_context_min_score": 0.4,
            "pre_context_max_results": 5,
            "auto_observe": True,
        },
    },
}

CATEGORIES = ("fresh", "grep", "reuse-probe")


def hermes_home() -> Path:
    env = os.environ.get("HERMES_HOME", "").strip()
    if env:
        return Path(env).expanduser()
    return Path.home() / ".hermes"


def load_tasks(path: Path) -> list[dict[str, Any]]:
    tasks = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        tasks.append(json.loads(line))
    return tasks


def base_config() -> dict[str, Any]:
    path = hermes_home() / "config.yaml"
    if not path.is_file():
        sys.exit(f"no config.yaml at {path}; run hermes setup first")
    return yaml.safe_load(path.read_text()) or {}


def prepare_home(
    work: Path, name: str, config: dict[str, Any], *, link_auth: bool
) -> Path:
    """A temporary Hermes home for one configuration.

    Derived from the operator's config.yaml. auth.json is symlinked, not copied,
    so the credential never leaves the real home.
    """
    spec = CONFIGS[name]
    cfg = copy.deepcopy(config)
    plugins = cfg.setdefault("plugins", {})
    entries = plugins.setdefault("entries", {})
    enabled = list(plugins.get("enabled") or [])
    if spec["enabled"]:
        if "physis" not in enabled:
            enabled.append("physis")
        entries["physis"] = {"settings": dict(spec["settings"])}
    else:
        enabled = [item for item in enabled if item != "physis"]
        entries.pop("physis", None)
    plugins["enabled"] = enabled

    home = work / f"home-{name}"
    home.mkdir(parents=True, exist_ok=True)
    (home / "config.yaml").write_text(yaml.safe_dump(cfg, sort_keys=False))
    auth = hermes_home() / "auth.json"
    if link_auth and auth.is_file():
        target = home / "auth.json"
        if not target.exists():
            target.symlink_to(auth)
    # Hermes loads user plugins from <HERMES_HOME>/plugins. Without this the
    # config names a plugin that is not there: every config silently degrades to
    # plain Hermes, which is exactly how the first benchmark run was invalid.
    plugins_src = hermes_home() / "plugins"
    plugins_dst = home / "plugins"
    if plugins_src.is_dir() and not plugins_dst.exists():
        plugins_dst.symlink_to(plugins_src)
    return home


def child_env(home: Path) -> dict[str, str]:
    """The run environment: HERMES_HOME redirected, plugin state overrides removed.

    A PHYSIS_CORE_DIR or PHYSIS_REPLANT_STORE left in the shell would point both
    the pre-index and the agent at one shared store, destroying the per-config
    isolation the benchmark depends on.
    """
    env = {**os.environ, "HERMES_HOME": str(home)}
    for name in ("PHYSIS_CORE_DIR", "PHYSIS_REPLANT_STORE"):
        env.pop(name, None)
    return env


def pre_index(home: Path, repo: Path, timeout: int = 120) -> bool:
    """Run the plugin's own physis_observe, so a plugin-enabled config starts
    with the index the skill tells the agent to build. Plain has no plugin, so
    it gets no index - that asymmetry is the point of the comparison."""
    code = (
        "import sys, json;"
        f"sys.path.insert(0, {str(PLUGIN_DIR)!r});"
        "import tools;"
        "print(json.dumps(tools.physis_observe("
        f"{{'path': {str(repo)!r}, 'timeout': {int(timeout)}}})))"
    )
    try:
        proc = subprocess.run(
            [sys.executable, "-c", code], capture_output=True, text=True,
            env=child_env(home), timeout=timeout + 60,
        )
    except subprocess.TimeoutExpired:
        return False
    return proc.returncode == 0 and '"ok":true' in proc.stdout.lower().replace(" ", "")


def make_repo(root: Path, setup: list[dict[str, str]]) -> None:
    """A fresh git repository, so the canonical repository root resolves."""
    root.mkdir(parents=True, exist_ok=True)
    for item in setup:
        target = root / item["path"]
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(item["content"])
    subprocess.run(["git", "init", "-q"], cwd=root, check=False,
                   capture_output=True)
    subprocess.run(["git", "add", "-A"], cwd=root, check=False, capture_output=True)
    subprocess.run(
        ["git", "-c", "user.email=bench@example.com", "-c", "user.name=bench",
         "commit", "-q", "-m", "fixture"],
        cwd=root, check=False, capture_output=True,
    )


def count_marker(home: Path, marker: str) -> int:
    """Occurrences of *marker* in this home's session messages (injection check).

    ``pre_llm_call`` context is stamped onto the user message's ``api_content``
    sidecar, not its ``content`` - checking only ``content`` reported zero
    injections even when the hook had fired.
    """
    db = home / "state.db"
    if not db.is_file():
        return 0
    try:
        con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
        row = con.execute(
            "select count(*) from messages where content like ? or api_content like ?",
            (f"%{marker}%", f"%{marker}%"),
        ).fetchone()
        con.close()
    except sqlite3.Error:
        return 0
    return int(row[0]) if row else 0


def count_physis_calls(home: Path) -> int:
    """Physis/replant tool calls recorded in this home's session database.

    Completed calls live in the ``tool_name`` column of ``tool``-role messages;
    the ``tool_calls`` JSON on assistant rows only holds the pending batch. The
    first version of this read only the JSON and reported zero for every run,
    which hid the fact that the plugin had not loaded.
    """
    db = home / "state.db"
    if not db.is_file():
        return 0
    try:
        con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
        rows = con.execute(
            "select tool_name from messages where tool_name is not null and tool_name != ''"
        ).fetchall()
        con.close()
    except sqlite3.Error:
        return 0
    return sum(1 for (name,) in rows if str(name).startswith(("physis_", "replant_")))


def run_one(
    *, home: Path, task: dict[str, Any], work: Path, timeout: int
) -> dict[str, Any]:
    repo = work / task["id"]
    make_repo(repo, task.get("setup") or [])
    plugins_dir = home / "plugins"
    plugin_entries = len(list(plugins_dir.iterdir())) if plugins_dir.is_dir() else 0
    print(f"    home={home.name} plugins_entries={plugin_entries} "
          f"symlink={plugins_dir.is_symlink()}", flush=True)
    indexed = pre_index(home, repo) if CONFIGS[home.name.replace("home-", "")]["enabled"] else False
    usage = work / f"{task['id']}.usage.json"
    env = child_env(home)
    physis_before = count_physis_calls(home)
    injected_before = count_marker(home, "[Physis context]")
    started = time.time()
    try:
        proc = subprocess.run(
            ["hermes", "-z", task["prompt"],
             "--in", str(repo), "--usage-file", str(usage)],
            capture_output=True, text=True, timeout=timeout, env=env,
        )
        agent_ok = proc.returncode == 0
        output = (proc.stdout or "").strip()
        error = (proc.stderr or "").strip()
    except subprocess.TimeoutExpired:
        agent_ok, output, error = False, "", "timed out"
    elapsed = time.time() - started

    check = subprocess.run(
        task["check"], shell=True, cwd=repo, capture_output=True, text=True, timeout=120
    )
    payload: dict[str, Any] = {}
    if usage.is_file():
        try:
            payload = json.loads(usage.read_text())
        except (OSError, ValueError):
            payload = {}
    return {
        "task": task["id"],
        "category": task["category"],
        "config": home.name.replace("home-", ""),
        "agent_ok": agent_ok,
        "check_ok": check.returncode == 0,
        "pre_indexed": indexed,
        "seconds": round(elapsed, 2),
        "input_tokens": payload.get("input_tokens") or 0,
        "cache_read_tokens": payload.get("cache_read_tokens") or 0,
        "output_tokens": payload.get("output_tokens") or 0,
        "cost_usd": payload.get("estimated_cost_usd") or 0.0,
        "api_calls": payload.get("api_calls") or 0,
        "model": payload.get("model") or "",
        "physis_tool_calls": count_physis_calls(home) - physis_before,
        "pre_context_injections": count_marker(home, "[Physis context]") - injected_before,
        "response_chars": len(output),
        "error": error[-400:],
    }


def aggregate(rows: list[dict[str, Any]]) -> None:
    print(f"\n{'config':<8} {'category':<12} {'ok':>4} {'runs':>5} {'in tok':>9} "
          f"{'cache':>9} {'out tok':>8} {'cost $':>8} {'sec':>7} {'physis':>7}")
    print("-" * 92)
    for config in ("plain", "skill", "auto"):
        for category in CATEGORIES:
            group = [r for r in rows if r["config"] == config and r["category"] == category]
            if not group:
                continue
            passed = sum(1 for r in group if r["check_ok"])
            print(
                f"{config:<8} {category:<12} {passed:>4} {len(group):>5} "
                f"{sum(r['input_tokens'] for r in group):>9} "
                f"{sum(r['cache_read_tokens'] for r in group):>9} "
                f"{sum(r['output_tokens'] for r in group):>8} "
                f"{sum(float(r['cost_usd']) for r in group):>8.4f} "
                f"{sum(r['seconds'] for r in group):>7.1f} "
                f"{sum(int(r.get('physis_tool_calls', 0)) for r in group):>7}"
            )
    print("-" * 92)
    for config in ("plain", "skill", "auto"):
        group = [r for r in rows if r["config"] == config]
        if not group:
            continue
        passed = sum(1 for r in group if r["check_ok"])
        print(f"{config:<8} {'TOTAL':<12} {passed:>4} {len(group):>5}")
    print("\ncheck_ok = the task's own verification command passed.")
    print("physis = physis/replant tool calls in the run's session database.")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--tasks", default=str(DEFAULT_TASKS))
    parser.add_argument("--configs", default="plain,skill,auto")
    parser.add_argument("--only", default="", help="comma-separated task ids")
    parser.add_argument("--out", default=str(HERE / "results.jsonl"))
    parser.add_argument("--timeout", type=int, default=300,
                        help="per-run agent timeout in seconds (default 300; the free "
                             "model occasionally loops, and an unbounded run makes the "
                             "benchmark unusable)")
    parser.add_argument("--no-pre-index", action="store_true",
                        help="do not build the graph before plugin-enabled runs")
    parser.add_argument("--no-link-auth", action="store_true",
                        help="do not symlink auth.json into the temporary homes")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    tasks = load_tasks(Path(args.tasks))
    if args.only:
        wanted = {item.strip() for item in args.only.split(",") if item.strip()}
        tasks = [t for t in tasks if t["id"] in wanted]
    configs = [name.strip() for name in args.configs.split(",") if name.strip()]
    unknown = [name for name in configs if name not in CONFIGS]
    if unknown:
        sys.exit(f"unknown config(s): {', '.join(unknown)}")

    home = hermes_home()
    config = base_config()
    print(f"tasks: {len(tasks)}  configs: {', '.join(configs)}  home: {home}")
    if args.dry_run:
        for name in configs:
            spec = CONFIGS[name]
            state = "enabled" if spec["enabled"] else "disabled"
            print(f"  home-{name:<6} physis {state:<8} settings={spec['settings'] or '{}'}")
        for task in tasks:
            print(f"  {task['id']:<12} [{task['category']}] {task['prompt'][:70]}")
        print("\ndry run: no model calls made.")
        return 0

    out_path = Path(args.out)
    rows: list[dict[str, Any]] = []
    # The work dir must stay outside HERMES_HOME: a home nested under the real
    # one makes the one-shot CLI discover and reuse the host gateway instead of
    # starting a local one for the temporary home.
    work_root = Path(os.environ.get("PHYSIS_BENCH_DIR", "/tmp"))
    work = Path(tempfile.mkdtemp(prefix="physis-bench-", dir=str(work_root)))
    try:
        for name in configs:
            bench_home = prepare_home(work, name, config, link_auth=not args.no_link_auth)
            for task in tasks:
                print(f"[{name}] {task['id']} ...", flush=True)
                row = run_one(
                    home=bench_home, task=task, work=work, timeout=args.timeout
                )
                if args.no_pre_index:
                    row["pre_indexed"] = False
                rows.append(row)
                with out_path.open("a") as handle:
                    handle.write(json.dumps(row) + "\n")
                print(f"    check={'ok' if row['check_ok'] else 'FAIL'} "
                      f"indexed={row['pre_indexed']} {row['seconds']}s "
                      f"in={row['input_tokens']} cache={row['cache_read_tokens']} "
                      f"out={row['output_tokens']}", flush=True)
                if row.get("error"):
                    print(f"    stderr: {row['error']}", flush=True)
    finally:
        # A run may leave a gateway or MCP server holding the home open; remove
        # what we can and say so rather than failing the whole benchmark.
        shutil.rmtree(work, ignore_errors=True)
        if work.exists():
            print(f"note: {work} could not be removed (a gateway or MCP server "
                  f"may still hold it); stop it with `hermes gateway stop`.")
    aggregate(rows)
    print(f"\nresults: {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
