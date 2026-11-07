"""Tool handlers for the Physis Hermes plugin.

The handlers are deliberately thin: they validate arguments, call the engine
command that actually exists, and parse the engine's own output into fields a
model can use. No handler invents evidence the engine did not return.
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path
from typing import Any

try:  # imported as hermes_plugins.<slug>.tools
    from .engine import EngineError, EnginePool, discover_engines
except ImportError:  # pragma: no cover - direct import from the plugin directory
    from engine import EngineError, EnginePool, discover_engines

TOOLSET = "physis"
DEFAULT_STORE = ".replant"

DEFAULTS: dict[str, Any] = {
    "engine_bin": "",
    "timeout": 20,
    "budget": 800,
    "auto_observe": False,
}

_POOL: EnginePool | None = None
_POOL_KEY: tuple[str, int] | None = None

_SCAN_RE = re.compile(r"Registered (\d+) files?\.\s*Nodes:\s*(\d+)")
_COHERENCE_RE = re.compile(r"Coherence index:\s*([0-9.]+)")
_SNAPSHOT_NODES_RE = re.compile(r"nodes:\s*(\d+)")
_SNAPSHOT_COHERENCE_RE = re.compile(r"coherence index:\s*([0-9.]+)")
_SNAPSHOT_HIGH_RE = re.compile(r"high/mid/low:\s*(\d+)/(\d+)/(\d+)")
_CELL_RE = re.compile(r"^\s{2}([A-Z][A-Z_-]*)\s+[×x]\s+([A-Z][A-Z_-]*)\s+([0-9.]+)")
_MARGIN_RE = re.compile(r"Margin over runner-up:\s*([0-9.]+)")


# --------------------------------------------------------------------------- #
# Configuration / plumbing
# --------------------------------------------------------------------------- #

def _merge(args: Any, kwargs: dict[str, Any]) -> dict[str, Any]:
    if isinstance(args, dict):
        merged = dict(args)
    elif args is None:
        merged = {}
    else:
        merged = {"value": args}
    merged.update(kwargs)
    return merged


def _with_defaults(merged: dict[str, Any]) -> dict[str, Any]:
    for key, value in DEFAULTS.items():
        merged.setdefault(key, value)
    return merged


def plugin_config(ctx: Any) -> dict[str, Any]:
    """Read ``plugins.entries.physis.settings`` through PluginContext."""
    config = dict(DEFAULTS)
    getter = getattr(ctx, "get_config", None)
    if callable(getter):
        for key, default in DEFAULTS.items():
            try:
                value = getter(key, default)
            except Exception:
                value = default
            if value is not None:
                config[key] = value
    # PHYSIS_BIN remains a useful escape hatch for operators.
    if not config.get("engine_bin"):
        config["engine_bin"] = os.environ.get("PHYSIS_BIN", "")
    return config


def _pool_from_args(args: dict[str, Any]) -> EnginePool:
    global _POOL, _POOL_KEY
    binary = str(args.get("engine_bin") or args.get("engine_bins") or "")
    try:
        timeout = int(args.get("timeout") or DEFAULTS["timeout"])
    except (TypeError, ValueError):
        timeout = int(DEFAULTS["timeout"])
    key = (binary, timeout)
    if _POOL is None or key != _POOL_KEY:
        _POOL = discover_engines(binary=binary or None, timeout=timeout)
        _POOL_KEY = key
    return _POOL


def reset_pool() -> None:
    """Drop the cached pool (tests, or after installing a new engine)."""
    global _POOL, _POOL_KEY
    _POOL = None
    _POOL_KEY = None


def _path(args: dict[str, Any]) -> str:
    raw = str(args.get("path") or ".").strip() or "."
    return str(Path(raw).expanduser().resolve())


def _store(args: dict[str, Any]) -> str:
    raw = str(args.get("store") or "").strip()
    if raw:
        return str(Path(raw).expanduser().resolve())
    return str(Path(_path(args)) / DEFAULT_STORE)


def _run(
    pool: EnginePool,
    argv: list[str],
    args: dict[str, Any],
    stdin: str | None = None,
    env: dict[str, str] | None = None,
    engine: str | None = None,
) -> dict[str, Any]:
    return pool.run(
        argv,
        cwd=_path(args),
        timeout=args.get("timeout"),
        stdin=stdin,
        env=env,
        engine=engine,
    )


def _unsupported(pool: EnginePool, command: str) -> dict[str, Any]:
    return {
        "ok": False,
        "error": f"no available engine advertises '{command}'",
        "supported": sorted(pool.capabilities()),
        "engines": [engine.binary for engine in pool.engines],
    }


def _engine_error(exc: EngineError) -> dict[str, Any]:
    return {"ok": False, "error": str(exc), "hint": "run physis_engine_status"}


def _json_payload(result: dict[str, Any]) -> dict[str, Any] | None:
    text = (result.get("stdout") or "").strip()
    if not text:
        return None
    try:
        payload = json.loads(text)
    except (ValueError, TypeError):
        return None
    return payload if isinstance(payload, dict) else None


def _copy_keys(result: dict[str, Any], payload: dict[str, Any] | None) -> None:
    if not payload:
        return
    for key in ("operation", "status", "provider_calls", "reason", "recipe_hash", "calls"):
        if key in payload:
            result[key] = payload[key]


# --------------------------------------------------------------------------- #
# physis-core tools
# --------------------------------------------------------------------------- #

def _graph_summary(pool: EnginePool, args: dict[str, Any]) -> dict[str, Any]:
    """Cheap snapshot used by status; never fails the caller."""
    if not pool.supports("snapshot"):
        return {}
    result = _run(pool, ["snapshot"], args)
    return _parse_snapshot(result.get("stdout", "")) or {}


def _parse_snapshot(text: str) -> dict[str, Any]:
    summary: dict[str, Any] = {}
    match = _SNAPSHOT_NODES_RE.search(text)
    if match:
        summary["nodes"] = int(match.group(1))
    match = _SNAPSHOT_COHERENCE_RE.search(text)
    if match:
        summary["coherence_index"] = float(match.group(1))
    match = _SNAPSHOT_HIGH_RE.search(text)
    if match:
        summary["high"], summary["mid"], summary["low"] = (int(g) for g in match.groups())
    return summary


def physis_engine_status(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    info = pool.info()
    info["ok"] = True
    info["store"] = os.environ.get("PHYSIS_CORE_DIR") or str(Path.home() / ".physis")
    info["graph"] = _graph_summary(pool, merged)
    return info


def physis_observe(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("scan"):
        return _unsupported(pool, "scan")

    path = _path(merged)
    if not Path(path).is_dir():
        return {"ok": False, "error": f"not a directory: {path}"}
    home = Path.home().resolve()
    if Path(path) == Path("/"):
        return {"ok": False, "error": "refusing to scan the filesystem root; pass a repository path"}
    if Path(path) == home:
        return {
            "ok": False,
            "error": "refusing to scan the whole home directory; pass a project subdirectory",
        }

    result = _run(pool, ["scan", path], merged)
    text = result.get("stdout", "")
    match = _SCAN_RE.search(text)
    if match:
        result["files"] = int(match.group(1))
        result["nodes"] = int(match.group(2))
    coherence = _COHERENCE_RE.search(text)
    if coherence:
        result["coherence_index"] = float(coherence.group(1))
    result["scanned_path"] = path
    return result


def physis_snapshot(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("snapshot"):
        return _unsupported(pool, "snapshot")
    result = _run(pool, ["snapshot"], merged)
    result.update(_parse_snapshot(result.get("stdout", "")))
    return result


def physis_context(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("node-search"):
        return _unsupported(pool, "node-search")

    task = str(merged.get("task") or merged.get("query") or "").strip()
    if not task:
        return {"ok": False, "error": "task is required (the question the packet must support)"}
    try:
        budget = int(merged.get("budget") or DEFAULTS["budget"])
    except (TypeError, ValueError):
        budget = int(DEFAULTS["budget"])
    budget = min(4000, max(100, budget))
    try:
        limit = max(1, min(50, int(merged.get("max") or 12)))
    except (TypeError, ValueError):
        limit = 12

    result = _run(
        pool,
        ["node-search", task, "--budget", str(budget), "--max", str(limit)],
        merged,
    )
    result["budget"] = budget
    if result.get("ok") and "No recalled nodes" in (result.get("stdout") or ""):
        result["hint"] = "index is empty: run physis_observe on the repository first"
    return result


def physis_query(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("search"):
        return _unsupported(pool, "search")

    question = str(merged.get("question") or merged.get("query") or "").strip()
    if not question:
        return {"ok": False, "error": "question is required"}
    try:
        limit = max(1, min(50, int(merged.get("limit") or 10)))
    except (TypeError, ValueError):
        limit = 10
    result = _run(pool, ["search", question, "--max", str(limit)], merged)
    if result.get("ok") and "No recalled nodes" in (result.get("stdout") or ""):
        result["hint"] = "index is empty: run physis_observe on the repository first"
    return result


def physis_why(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("node-search"):
        return _unsupported(pool, "node-search")

    obj = str(merged.get("object") or merged.get("label") or "").strip()
    if not obj:
        return {"ok": False, "error": "object is required"}
    try:
        budget = int(merged.get("budget") or 600)
    except (TypeError, ValueError):
        budget = 600
    budget = min(4000, max(100, budget))
    result = _run(
        pool, ["node-search", obj, "--budget", str(budget), "--max", "8"], merged
    )
    result["note"] = (
        "Evidence retrieval: nodes the engine associates with the label. "
        "Absence of the object is real missing evidence, not proof of anything else."
    )
    return result


def _read_text(path: str, limit: int = 120_000) -> str:
    data = Path(path).read_bytes()
    text = data.decode("utf-8", errors="replace")
    return text[:limit]


def _parse_cells(text: str) -> list[dict[str, Any]]:
    cells: list[dict[str, Any]] = []
    for line in text.splitlines():
        match = _CELL_RE.match(line)
        if match:
            domain, mode, score = match.groups()
            cells.append({"cell": f"{domain}×{mode}", "score": float(score)})
    return cells


def physis_classify(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("classify"):
        return _unsupported(pool, "classify")

    text = merged.get("text")
    if text is None and merged.get("file"):
        file_path = Path(str(merged["file"])).expanduser()
        if not file_path.is_file():
            return {"ok": False, "error": f"file not found: {file_path}"}
        text = _read_text(str(file_path))
    text = str(text or "").strip()
    if not text:
        return {"ok": False, "error": "text or file is required"}

    result = _run(pool, ["classify", text], merged)
    cells = _parse_cells(result.get("stdout", ""))
    if cells:
        result["cells"] = cells[:8]
        result["top_cell"] = cells[0]
    margin = _MARGIN_RE.search(result.get("stdout", ""))
    if margin:
        result["margin"] = float(margin.group(1))
    result["tie"] = "effectively a tie" in (result.get("stdout") or "")
    return result


def physis_diff(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    """Structural delta between two files using the engine's classifier."""
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("classify"):
        return _unsupported(pool, "classify")

    before = str(merged.get("before") or "").strip()
    after = str(merged.get("after") or "").strip()
    if not before or not after:
        return {"ok": False, "error": "before and after file paths are required"}
    paths = [Path(before).expanduser(), Path(after).expanduser()]
    for item in paths:
        if not item.is_file():
            return {"ok": False, "error": f"file not found: {item}"}

    outputs = []
    for item in paths:
        result = _run(pool, ["classify", _read_text(str(item))], merged)
        outputs.append(result)
    if not all(item.get("ok") for item in outputs):
        return {
            "ok": False,
            "error": "classification failed",
            "before": outputs[0],
            "after": outputs[1],
        }

    before_cells = _parse_cells(outputs[0].get("stdout", ""))
    after_cells = _parse_cells(outputs[1].get("stdout", ""))
    before_map = {cell["cell"]: cell["score"] for cell in before_cells}
    after_map = {cell["cell"]: cell["score"] for cell in after_cells}
    added = sorted(set(after_map) - set(before_map))
    removed = sorted(set(before_map) - set(after_map))
    return {
        "ok": True,
        "before": before,
        "after": after,
        "top_before": before_cells[0] if before_cells else None,
        "top_after": after_cells[0] if after_cells else None,
        "top_changed": bool(
            before_cells and after_cells
            and before_cells[0]["cell"] != after_cells[0]["cell"]
        ),
        "cells_added": added,
        "cells_removed": removed,
        "cells_before": before_cells[:8],
        "cells_after": after_cells[:8],
        "note": "Structural profile change, not a line-by-line diff.",
    }


def physis_command(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)

    command = str(merged.get("command") or "").strip()
    if not command:
        return {"ok": False, "error": "command is required"}
    if not pool.supports(command):
        return _unsupported(pool, command)
    extra = merged.get("args") or []
    if isinstance(extra, str):
        extra = extra.split()
    stdin = merged.get("stdin")
    if stdin is not None and not isinstance(stdin, str):
        stdin = json.dumps(stdin)
    return _run(pool, [command, *[str(item) for item in extra]], merged, stdin=stdin)


# --------------------------------------------------------------------------- #
# replant tools
# --------------------------------------------------------------------------- #

def _replant_ready(pool: EnginePool) -> dict[str, Any] | None:
    needed = {"run", "record", "stats", "show"}
    missing = sorted(needed - pool.capabilities())
    if missing:
        return {
            "ok": False,
            "error": f"bundled replant engine does not advertise: {', '.join(missing)}",
            "supported": sorted(pool.capabilities()),
        }
    return None


def replant_run(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("run"):
        return _unsupported(pool, "run")

    value = str(merged.get("input") or merged.get("value") or "").strip()
    if not value:
        return {"ok": False, "error": "input is required"}
    tools = str(merged.get("tools") or "").strip()
    if not tools:
        return {"ok": False, "error": "tools registry path is required (tools=...)"}
    if not Path(tools).expanduser().is_file():
        return {"ok": False, "error": f"tools registry not found: {tools}"}

    stdin = json.dumps({"prompt": value, "input": {}, "context": {}})
    result = _run(
        pool,
        ["run", "--tools", str(Path(tools).expanduser()), "--store", _store(merged)],
        merged,
        stdin=stdin,
        engine="replant",
    )
    _copy_keys(result, _json_payload(result))
    return result


def replant_record(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("record"):
        return _unsupported(pool, "record")

    value = str(merged.get("input") or merged.get("value") or "").strip()
    if not value:
        return {"ok": False, "error": "input is required"}
    recipe = merged.get("recipe")
    if not recipe:
        return {"ok": False, "error": "recipe is required"}
    if isinstance(recipe, str):
        try:
            recipe = json.loads(recipe)
        except (ValueError, TypeError):
            return {"ok": False, "error": "recipe must be valid JSON"}
    tools = str(merged.get("tools") or "").strip()
    if not tools:
        return {"ok": False, "error": "tools registry path is required (tools=...)"}
    if not Path(tools).expanduser().is_file():
        return {"ok": False, "error": f"tools registry not found: {tools}"}

    trace = merged.get("trace") or {"provider": "hermes-physis-plugin"}
    if isinstance(trace, str):
        try:
            trace = json.loads(trace)
        except (ValueError, TypeError):
            return {"ok": False, "error": "trace must be valid JSON"}
    stdin = json.dumps(
        {
            "request": {"prompt": value, "input": {}, "context": {}},
            "recipe": recipe,
            "trace": trace,
        }
    )
    result = _run(
        pool,
        ["record", "--tools", str(Path(tools).expanduser()), "--store", _store(merged)],
        merged,
        stdin=stdin,
        engine="replant",
    )
    _copy_keys(result, _json_payload(result))
    return result


def replant_stats(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("stats"):
        return _unsupported(pool, "stats")
    result = _run(pool, ["stats", "--store", _store(merged)], merged, stdin="{}", engine="replant")
    payload = _json_payload(result)
    if payload:
        result.update(payload)
    return result


def _write_demo_registry(store: Path) -> tuple[Path, Path]:
    """Create a self-contained counter fixture + registry inside *store*."""
    store.mkdir(parents=True, exist_ok=True, mode=0o700)
    try:
        os.chmod(store, 0o700)  # replant refuses a state dir other users can read
    except OSError:
        pass
    fixture = Path(__file__).resolve().parent / "examples" / "replant_fixture.py"
    if not fixture.is_file():
        raise FileNotFoundError(f"bundled fixture missing: {fixture}")
    registry = store / "tools.json"
    root = store / "fixture"
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    payload = {
        "increment": {
            "kind": "action",
            "program": sys.executable,
            "args": [str(fixture), "increment", str(root)],
            "fingerprint_files": [str(fixture)],
            "timeout_ms": 5000,
        },
        "verify": {
            "kind": "check",
            "program": sys.executable,
            "args": [str(fixture), "verify", str(root)],
            "fingerprint_files": [str(fixture)],
            "timeout_ms": 5000,
        },
    }
    registry.write_text(json.dumps(payload, indent=2))
    return registry, root


def replant_demo(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    blocked = _replant_ready(pool)
    if blocked:
        return blocked

    store = Path(_store(merged))
    try:
        registry, fixture_root = _write_demo_registry(store)
    except OSError as exc:
        return {"ok": False, "error": str(exc)}

    request = {
        "prompt": "physis replant demo",
        "input": {},
        "context": {"workflow": "physis-plugin-demo"},
    }
    recipe = {
        "steps": [{"tool": "increment", "input": {"amount": 1}}],
        "checks": [{"tool": "verify", "input": {}}],
    }
    record_stdin = json.dumps(
        {"request": request, "recipe": recipe, "trace": {"provider": "hermes-physis-plugin"}}
    )
    cli = ["--tools", str(registry), "--store", str(store)]
    record = _run(pool, ["record", *cli], merged, stdin=record_stdin, engine="replant")
    first = _run(pool, ["run", *cli], merged, stdin=json.dumps(request), engine="replant")
    second = _run(pool, ["run", *cli], merged, stdin=json.dumps(request), engine="replant")
    stats = _run(pool, ["stats", "--store", str(store)], merged, stdin="{}", engine="replant")

    record_payload = _json_payload(record) or {}
    first_payload = _json_payload(first) or {}
    second_payload = _json_payload(second) or {}
    stats_payload = _json_payload(stats) or {}
    counter_file = fixture_root / "counter"
    counter_value = counter_file.read_text().strip() if counter_file.exists() else None

    return {
        "ok": bool(record.get("ok") and second.get("ok")),
        "store": str(store),
        "counter": counter_value,
        "record_status": record_payload.get("status"),
        "record_operation": record_payload.get("operation"),
        "first_operation": first_payload.get("operation"),
        "second_operation": second_payload.get("operation"),
        "replayed": second_payload.get("operation") == "replay",
        "provider_calls": second_payload.get("provider_calls"),
        "stats": stats_payload,
        "note": "The action and check really ran on every pass; only the planner was skipped.",
    }


def replant_show(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("show"):
        return _unsupported(pool, "show")
    value = str(merged.get("input") or merged.get("value") or "").strip()
    if not value:
        return {"ok": False, "error": "input is required"}
    stdin = json.dumps({"prompt": value, "input": {}, "context": {}})
    result = _run(pool, ["show", "--store", _store(merged)], merged, stdin=stdin, engine="replant")
    payload = _json_payload(result)
    if payload:
        result.update(payload)
    return result


HANDLERS = {
    "physis_engine_status": physis_engine_status,
    "physis_observe": physis_observe,
    "physis_snapshot": physis_snapshot,
    "physis_context": physis_context,
    "physis_query": physis_query,
    "physis_why": physis_why,
    "physis_diff": physis_diff,
    "physis_classify": physis_classify,
    "physis_command": physis_command,
    "replant_run": replant_run,
    "replant_record": replant_record,
    "replant_stats": replant_stats,
    "replant_demo": replant_demo,
    "replant_show": replant_show,
}


# --------------------------------------------------------------------------- #
# Hermes registration
# --------------------------------------------------------------------------- #

def register_tools(ctx: Any) -> list[str]:
    """Register every declared tool on ``PluginContext``.

    Hermes' signature is ``register_tool(name, toolset, schema, handler)``.
    The toolset groups the tools in the UI; settings are captured per call so a
    config edit applies without reloading the plugin.
    """
    try:
        from .schemas import ALL_SCHEMAS
    except ImportError:  # pragma: no cover
        from schemas import ALL_SCHEMAS

    registered: list[str] = []
    for schema in ALL_SCHEMAS:
        name = schema["name"]
        handler = HANDLERS[name]

        def wrapped(args=None, _handler=handler, **kwargs):
            merged = _with_defaults(_merge(args, kwargs))
            # Lazy so tests and re-registration read current config.
            for key in ("engine_bin", "timeout"):
                if not merged.get(key) and DEFAULTS[key]:
                    merged[key] = DEFAULTS[key]
            return _handler(merged)

        ctx.register_tool(
            name,
            TOOLSET,
            schema,
            wrapped,
            description=schema.get("description", ""),
        )
        registered.append(name)
    return registered


def make_post_tool_call(config: dict[str, Any]):
    """Incremental, bounded re-index after a file write when auto_observe is on."""
    def _hook(tool_name=None, tool_args=None, result=None, **kwargs):
        try:
            if not config.get("auto_observe"):
                return None
            if tool_name not in {"write_file", "patch", "edit_file", "apply_patch"}:
                return None
            args = tool_args if isinstance(tool_args, dict) else {}
            target = args.get("path") or args.get("file_path")
            if not target:
                return None
            folder = str(Path(target).expanduser().resolve().parent)
            pool = _pool_from_args(
                {
                    "engine_bin": config.get("engine_bin", ""),
                    "timeout": min(int(config.get("timeout") or 20), 8),
                }
            )
            if not pool.supports("watch"):
                return None
            pool.run(
                ["watch", "--source", "fs", "--path", folder, "--max", "200"],
                cwd=folder,
                timeout=min(int(config.get("timeout") or 20), 8),
            )
        except Exception:
            return None
        return None

    return _hook
