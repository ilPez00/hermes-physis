"""Tool handlers for the Physis Hermes plugin."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

from engine import EnginePool, discover_engines, EngineError

_DEFAULT_POOL: EnginePool | None = None


def _merge(args: Any, kwargs: dict[str, Any]) -> dict[str, Any]:
    if isinstance(args, dict):
        merged = dict(args)
    elif args is None:
        merged = {}
    else:
        merged = {"value": args}
    merged.update(kwargs)
    return merged


def _pool_from_args(args: dict[str, Any]) -> EnginePool:
    global _DEFAULT_POOL
    binary = args.get("engine_bin") or args.get("engine_bins")
    timeout = args.get("timeout", 15)
    if binary or _DEFAULT_POOL is None:
        _DEFAULT_POOL = discover_engines(binary=binary, timeout=timeout)
    else:
        _DEFAULT_POOL.timeout = max(1, int(timeout or _DEFAULT_POOL.timeout))
        for engine in _DEFAULT_POOL.engines:
            engine.timeout = _DEFAULT_POOL.timeout
    return _DEFAULT_POOL


# Backwards-compatible alias used by tests and older callers.
_engine_from_args = _pool_from_args


def _unsupported(pool: EnginePool, command: str) -> dict[str, Any]:
    return {
        "ok": False,
        "error": f"no available engine advertises '{command}'",
        "supported": sorted(pool.capabilities()),
        "engines": [e.binary for e in pool.engines],
    }


def _path(args: dict[str, Any]) -> str:
    return str(args.get("path") or ".")


def _run(pool: EnginePool, args: list[str], merged: dict[str, Any], stdin: str | None = None) -> dict[str, Any]:
    return pool.run(args, cwd=_path(merged), timeout=merged.get("timeout"), stdin=stdin)


def physis_engine_status(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    return {"ok": True, **pool.info()}


def physis_observe(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("scan"):
        return _unsupported(pool, "scan")
    cli_args = ["scan", _path(merged)]
    return _run(pool, cli_args, merged)


def physis_context(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    path = _path(merged)
    budget = int(merged.get("budget", 800) or 800)
    budget = min(4000, max(100, budget))
    task = (merged.get("task") or "").strip()
    if not pool.supports("context"):
        return _unsupported(pool, "context")
    query = task or path
    return _run(pool, ["context", "--corpus", path, "--query", query, "--budget", str(budget)], merged)


def physis_query(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("search"):
        return _unsupported(pool, "search")
    question = str(merged.get("question") or "").strip()
    if not question:
        return {"ok": False, "error": "question is required"}
    path = _path(merged)
    cli_args = ["search", question]
    limit = merged.get("limit")
    if limit is not None:
        cli_args += ["--max", str(int(limit))]
    return pool.run(cli_args, cwd=path, timeout=merged.get("timeout"))


def physis_why(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("ground"):
        return _unsupported(pool, "ground")
    obj = str(merged.get("object") or "").strip()
    if not obj:
        return {"ok": False, "error": "object is required"}
    return _run(pool, ["ground"], merged)


def physis_diff(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("direction"):
        return _unsupported(pool, "direction")
    from_ref = str(merged.get("from_ref") or "HEAD~1")
    to_ref = str(merged.get("to_ref") or "HEAD")
    result = _run(pool, ["direction", "--before", from_ref, "--after", to_ref], merged)
    # direction returns exit 1 on a FAILS verdict — still valid output
    if not result.get("ok") and result.get("stdout"):
        result["ok"] = True
        result["note"] = "direction verdict produced (non-zero exit = FAILS verdict, not engine error)"
    return result


def physis_command(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    """Run any command advertised by an available engine."""
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    command = str(merged.get("command") or "").strip()
    if not command:
        return {"ok": False, "error": "command is required"}
    if not pool.supports(command):
        return _unsupported(pool, command)
    extra = merged.get("args") or []
    if isinstance(extra, str):
        extra = extra.split()
    cli_args = [command, *[str(item) for item in extra]]
    return _run(pool, cli_args, merged)


def replant_run(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("run"):
        return _unsupported(pool, "run")
    value = str(merged.get("input") or merged.get("value") or "").strip()
    if not value:
        return {"ok": False, "error": "input is required"}
    tools = merged.get("tools")
    store = merged.get("store") or f"{_path(merged)}/.physis/replant"
    if not tools:
        return {"ok": False, "error": "replant_run requires a tools registry path (tools=...)"}
    cli_args = ["run", "--tools", tools, "--store", store]
    stdin = json.dumps({"prompt": value, "input": {}, "context": {}})
    result = _run(pool, cli_args, merged, stdin=stdin)
    if result.get("ok") and result.get("stdout"):
        try:
            payload = json.loads(result["stdout"])
            result.update({k: payload.get(k) for k in ("operation", "status", "provider_calls", "reason", "recipe_hash") if k in payload})
        except (json.JSONDecodeError, ValueError):
            pass
    return result


def replant_record(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("record"):
        return _unsupported(pool, "record")
    value = str(merged.get("input") or merged.get("value") or "").strip()
    recipe = merged.get("recipe")
    if not value:
        return {"ok": False, "error": "input is required"}
    if not recipe:
        return {"ok": False, "error": "recipe is required"}
    # recipe may arrive as a JSON string; parse it so it isn't double-encoded.
    if isinstance(recipe, str):
        try:
            recipe = json.loads(recipe)
        except (json.JSONDecodeError, ValueError):
            return {"ok": False, "error": "recipe must be valid JSON"}
    tools = merged.get("tools")
    store = merged.get("store") or f"{_path(merged)}/.physis/replant"
    if not tools:
        return {"ok": False, "error": "replant_record requires a tools registry path (tools=...)"}
    cli_args = ["record", "--tools", tools, "--store", store]
    stdin = json.dumps({"request": {"prompt": value, "input": {}, "context": {}}, "recipe": recipe, "trace": merged.get("trace") or {"provider": "hermes-physis-plugin"}})
    result = _run(pool, cli_args, merged, stdin=stdin)
    if result.get("ok") and result.get("stdout"):
        try:
            payload = json.loads(result["stdout"])
            result.update({k: payload.get(k) for k in ("operation", "status", "provider_calls", "reason", "recipe_hash") if k in payload})
        except (json.JSONDecodeError, ValueError):
            pass
    return result


def replant_stats(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("stats"):
        return _unsupported(pool, "stats")
    store = merged.get("store") or f"{_path(merged)}/.physis/replant"
    cli_args = ["stats", "--store", store]
    stdin = json.dumps({})
    result = _run(pool, cli_args, merged, stdin=stdin)
    if result.get("ok") and result.get("stdout"):
        try:
            result.update(json.loads(result["stdout"]))
        except (json.JSONDecodeError, ValueError):
            pass
    return result


def replant_demo(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("run") or not pool.supports("record") or not pool.supports("stats"):
        return _unsupported(pool, "run")
    tools = merged.get("tools")
    store = merged.get("store") or f"{_path(merged)}/.physis/replant"
    if not tools:
        return {"ok": False, "error": "replant_demo requires a tools registry path (tools=...)"}
    recipe = {"steps": [{"tool": "increment", "input": {"amount": 1}}], "checks": [{"tool": "verify", "input": {}}]}
    # 1. Record the recipe
    record_stdin = json.dumps({"request": {"prompt": "replant demo", "input": {}, "context": {}}, "recipe": recipe, "trace": {"provider": "hermes-physis-plugin"}})
    record_result = _run(pool, ["record", "--tools", tools, "--store", store], merged, stdin=record_stdin)
    # 2. Run same input → should replay
    run_stdin = json.dumps({"prompt": "replant demo", "input": {}, "context": {}})
    cli = ["run", "--tools", tools, "--store", store]
    first = _run(pool, cli, merged, stdin=run_stdin)
    second = _run(pool, cli, merged, stdin=run_stdin)
    # Parse JSON from stdout for operation extraction.
    def _op(res):
        if res.get("ok") and res.get("stdout"):
            try:
                return json.loads(res["stdout"]).get("operation")
            except (json.JSONDecodeError, ValueError):
                return None
        return None
    stats = _run(pool, ["stats", "--store", store], merged, stdin=json.dumps({}))
    return {
        "ok": True,
        "record_operation": record_result.get("operation"),
        "first_operation": _op(first),
        "second_operation": _op(second),
        "cache_hit": _op(second) == "replay",
        "stats": stats,
    }


def replant_show(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("show"):
        return _unsupported(pool, "show")
    value = str(merged.get("input") or merged.get("value") or "").strip()
    store = merged.get("store") or f"{_path(merged)}/.physis/replant"
    cli_args = ["show", "--store", store]
    stdin = json.dumps({"prompt": value, "input": {}, "context": {}})
    result = _run(pool, cli_args, merged, stdin=stdin)
    if result.get("ok") and result.get("stdout"):
        try:
            result.update(json.loads(result["stdout"]))
        except (json.JSONDecodeError, ValueError):
            pass
    return result


HANDLERS = {
    "physis_engine_status": physis_engine_status,
    "physis_observe": physis_observe,
    "physis_context": physis_context,
    "physis_query": physis_query,
    "physis_why": physis_why,
    "physis_diff": physis_diff,
    "physis_command": physis_command,
    "replant_run": replant_run,
    "replant_record": replant_record,
    "replant_stats": replant_stats,
    "replant_demo": replant_demo,
    "replant_show": replant_show,
}
