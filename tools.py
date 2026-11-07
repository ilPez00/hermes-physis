"""Tool handlers for the Physis Hermes plugin."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any

from engine import PhysisEngine, EngineError

_DEFAULT_ENGINE: PhysisEngine | None = None


def _merge(args: Any, kwargs: dict[str, Any]) -> dict[str, Any]:
    if isinstance(args, dict):
        merged = dict(args)
    elif args is None:
        merged = {}
    else:
        merged = {"value": args}
    merged.update(kwargs)
    return merged


def _engine_from_args(args: dict[str, Any]) -> PhysisEngine:
    global _DEFAULT_ENGINE
    binary = args.get("engine_bin") or os.environ.get("PHYSIS_BIN")
    timeout = args.get("timeout", 15)
    if binary or _DEFAULT_ENGINE is None:
        try:
            _DEFAULT_ENGINE = PhysisEngine(binary=binary or None, timeout=timeout)
        except EngineError as exc:
            raise
    else:
        _DEFAULT_ENGINE.timeout = max(1, int(timeout or _DEFAULT_ENGINE.timeout))
    return _DEFAULT_ENGINE


def _unsupported(engine: PhysisEngine, command: str) -> dict[str, Any]:
    return {
        "ok": False,
        "error": f"engine {engine.binary} does not advertise '{command}'",
        "supported": sorted(engine.capabilities()),
    }


def _path(args: dict[str, Any]) -> str:
    return str(args.get("path") or ".")


def physis_engine_status(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        engine = _engine_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    return {"ok": True, **engine.info()}


def physis_observe(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        engine = _engine_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not engine.supports("observe"):
        return _unsupported(engine, "observe")
    cli_args = ["observe", _path(merged)]
    max_files = merged.get("max_files")
    if max_files is not None:
        cli_args += ["--max-files", str(int(max_files))]
    if merged.get("doc_comments"):
        cli_args.append("--doc-comments")
    for item in merged.get("excludes") or []:
        cli_args += ["--exclude", str(item)]
    return engine.run(cli_args, cwd=_path(merged), timeout=merged.get("timeout"))


def physis_context(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        engine = _engine_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    path = _path(merged)
    budget = int(merged.get("budget", 800) or 800)
    budget = min(4000, max(100, budget))
    task = (merged.get("task") or "").strip()
    if task and engine.supports("compile"):
        cli_args = ["compile", task, "--path", path, "--budget", str(budget)]
        return engine.run(cli_args, cwd=path, timeout=merged.get("timeout"))
    if not engine.supports("context"):
        return _unsupported(engine, "context")
    cli_args = ["context", path, "--budget", str(budget)]
    return engine.run(cli_args, cwd=path, timeout=merged.get("timeout"))


def physis_query(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        engine = _engine_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not engine.supports("query"):
        return _unsupported(engine, "query")
    question = str(merged.get("question") or "").strip()
    if not question:
        return {"ok": False, "error": "question is required"}
    path = _path(merged)
    cli_args = ["query", question]
    limit = merged.get("limit")
    if limit is not None:
        cli_args += ["--limit", str(int(limit))]
    return engine.run(cli_args, cwd=path, timeout=merged.get("timeout"))


def physis_why(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        engine = _engine_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not engine.supports("why"):
        return _unsupported(engine, "why")
    obj = str(merged.get("object") or "").strip()
    if not obj:
        return {"ok": False, "error": "object is required"}
    path = _path(merged)
    return engine.run(["why", obj], cwd=path, timeout=merged.get("timeout"))


def physis_diff(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _merge(args, kwargs)
    try:
        engine = _engine_from_args(merged)
    except EngineError as exc:
        return {"ok": False, "error": str(exc)}
    if not engine.supports("diff"):
        return _unsupported(engine, "diff")
    path = _path(merged)
    from_ref = str(merged.get("from_ref") or "HEAD~1")
    to_ref = str(merged.get("to_ref") or "HEAD")
    return engine.run(["diff", f"{from_ref}..{to_ref}"], cwd=path, timeout=merged.get("timeout"))


HANDLERS = {
    "physis_engine_status": physis_engine_status,
    "physis_observe": physis_observe,
    "physis_context": physis_context,
    "physis_query": physis_query,
    "physis_why": physis_why,
    "physis_diff": physis_diff,
}
