"""Tool handlers for the Physis Hermes plugin."""

from __future__ import annotations

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


def _run(pool: EnginePool, args: list[str], merged: dict[str, Any]) -> dict[str, Any]:
    return pool.run(args, cwd=_path(merged), timeout=merged.get("timeout"))


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
    if not pool.supports("observe"):
        return _unsupported(pool, "observe")
    cli_args = ["observe", _path(merged)]
    max_files = merged.get("max_files")
    if max_files is not None:
        cli_args += ["--max-files", str(int(max_files))]
    if merged.get("doc_comments"):
        cli_args.append("--doc-comments")
    for item in merged.get("excludes") or []:
        cli_args += ["--exclude", str(item)]
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
    if task and pool.supports("compile"):
        return _run(pool, ["compile", task, "--path", path, "--budget", str(budget)], merged)
    if not pool.supports("context"):
        return _unsupported(pool, "context")
    return _run(pool, ["context", path, "--budget", str(budget)], merged)


def physis_query(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("query"):
        return _unsupported(pool, "query")
    question = str(merged.get("question") or "").strip()
    if not question:
        return {"ok": False, "error": "question is required"}
    path = _path(merged)
    cli_args = ["query", question]
    limit = merged.get("limit")
    if limit is not None:
        cli_args += ["--limit", str(int(limit))]
    return pool.run(cli_args, cwd=path, timeout=merged.get("timeout"))


def physis_why(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("why"):
        return _unsupported(pool, "why")
    obj = str(merged.get("object") or "").strip()
    if not obj:
        return {"ok": False, "error": "object is required"}
    return _run(pool, ["why", obj], merged)


def physis_diff(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("diff"):
        return _unsupported(pool, "diff")
    from_ref = str(merged.get("from_ref") or "HEAD~1")
    to_ref = str(merged.get("to_ref") or "HEAD")
    return _run(pool, ["diff", f"{from_ref}..{to_ref}"], merged)


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
    return _run(pool, ["run", value], merged)


def replant_stats(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("stats"):
        return _unsupported(pool, "stats")
    return _run(pool, ["stats"], merged)


def replant_demo(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not pool.supports("demo"):
        return _unsupported(pool, "demo")
    return _run(pool, ["demo"], merged)


HANDLERS = {
    "physis_engine_status": physis_engine_status,
    "physis_observe": physis_observe,
    "physis_context": physis_context,
    "physis_query": physis_query,
    "physis_why": physis_why,
    "physis_diff": physis_diff,
    "physis_command": physis_command,
    "replant_run": replant_run,
    "replant_stats": replant_stats,
    "replant_demo": replant_demo,
}
