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
    "model_dir": "",
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
_GRID_LINE_RE = re.compile(r"Grid:\s*(\d+)×(\d+)")
_EMBEDDER_LINE_RE = re.compile(r"Embedder:\s*([^\s(]+)")


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


def _model_env(args: dict[str, Any]) -> dict[str, str] | None:
    model_dir = str(args.get("model_dir") or "").strip()
    if model_dir:
        return {"PHYSIS_MODEL_DIR": model_dir, "PHYSIS_MODEL_DIM": "384"}
    return None


def _run(
    pool: EnginePool,
    argv: list[str],
    args: dict[str, Any],
    stdin: str | None = None,
    env: dict[str, str] | None = None,
    engine: str | None = None,
) -> dict[str, Any]:
    call_env = _model_env(args)
    if env:
        call_env = {**(call_env or {}), **env}
    return pool.run(
        argv,
        cwd=_path(args),
        timeout=args.get("timeout"),
        stdin=stdin,
        env=call_env,
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


_NODE_SEARCH_RE = re.compile(r"Fixed-token search:\s*(\d+) results?\s*\((\d+) tokens, budget (\d+)\)")


_SKIP_DIRS = {".git", "target", "node_modules", "vendor", ".next", "dist", "build", ".venv", "__pycache__"}


def _count_files(root: str, limit: int) -> int:
    """Count files under *root*, stopping at *limit* + 1 (cheap guard)."""
    count = 0
    for _dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in _SKIP_DIRS]
        count += len(filenames)
        if count > limit:
            return count
    return count


def physis_context(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    """Fixed-token context packet.

    Two corpora behave differently: the coherence graph (scan) sees code and
    docs, while the `context` compiler only ingests markdown/text documents.
    Default `mode="code"` therefore uses the indexed graph (auto-scanning once
    if the store is empty); `mode="docs"` uses the compiler's JSON report.
    """
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)

    task = str(merged.get("task") or merged.get("query") or "").strip()
    if not task:
        return {"ok": False, "error": "task is required (the question the packet must support)"}
    path = _path(merged)
    if not Path(path).is_dir():
        return {"ok": False, "error": f"corpus directory not found: {path}"}
    try:
        budget = int(merged.get("budget") or DEFAULTS["budget"])
    except (TypeError, ValueError):
        budget = int(DEFAULTS["budget"])
    budget = min(4000, max(100, budget))
    mode = str(merged.get("mode") or "code").strip().lower()

    if mode == "docs":
        if not pool.supports("context"):
            return _unsupported(pool, "context")
        doc_files = _count_files(path, 5000)
        if doc_files > 5000:
            return {
                "ok": False,
                "error": f"corpus is too large for the docs compiler ({doc_files}+ files)",
                "hint": "point path at a docs/ subdirectory or use mode=code on an indexed subtree",
            }
        result = _run(
            pool,
            ["context", "--corpus", path, "--query", task, "--budget", str(budget), "--json"],
            merged,
        )
        payload = _json_payload(result)
        if payload:
            for key in (
                "source_documents",
                "baseline_tokens",
                "physis_tokens",
                "compression_ratio",
                "repeats",
                "differences",
                "contradictions",
                "clusters",
                "physis_context",
            ):
                if key in payload:
                    result[key] = payload[key]
        result["mode"] = "docs"
        result["task"] = task
        result["budget"] = budget
        return result

    if mode != "code":
        return {"ok": False, "error": "mode must be code or docs"}

    if not pool.supports("node-search"):
        return _unsupported(pool, "node-search")

    def _node_search() -> dict[str, Any]:
        return _run(
            pool,
            ["node-search", task, "--budget", str(budget), "--max", str(merged.get("max") or 12)],
            merged,
        )

    result = _node_search()
    text = result.get("stdout") or ""
    match = _NODE_SEARCH_RE.search(text)
    empty = "No recalled nodes" in text or (match is not None and int(match.group(1)) == 0)
    auto_indexed: bool | None = None
    if empty and pool.supports("scan"):
        # First use on an unindexed repo: index it once, then answer. The
        # engine reports an empty store as "0 results", not as an error.
        # A whole-tree scan of a huge repo is not a context call; refuse it
        # with an actionable message instead of hanging for minutes.
        file_count = _count_files(path, 1500)
        if file_count > 1500:
            result["too_large_to_auto_index"] = True
            result["hint"] = (
                f"index is empty and the corpus is large ({file_count}+ files); "
                "run physis_observe on a subdirectory, or pass a narrower path"
            )
        else:
            scanned = _run(pool, ["scan", path], merged)
            auto_indexed = bool(scanned.get("ok"))
            if scanned.get("ok"):
                result = _node_search()
                text = result.get("stdout") or ""
    if auto_indexed is not None:
        result["auto_indexed"] = auto_indexed
    if "No recalled nodes" in text:
        result["hint"] = "index is empty and auto-scan did not add nodes; run physis_observe on the repository"
    match = _NODE_SEARCH_RE.search(text)
    if match and int(match.group(1)) == 0 and not result.get("too_large_to_auto_index"):
        result["hint"] = "no node matched; try different wording or run physis_observe on a wider path"
    if match:
        result["results"], result["tokens"], result["budget_reported"] = (
            int(match.group(1)),
            int(match.group(2)),
            int(match.group(3)),
        )
    result["mode"] = "code"
    result["task"] = task
    result["budget"] = budget
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
    shape = _GRID_LINE_RE.search(result.get("stdout", ""))
    if shape:
        result["grid_shape"] = [int(shape.group(1)), int(shape.group(2))]
    embedder = _EMBEDDER_LINE_RE.search(result.get("stdout", ""))
    if embedder:
        result["embedder_kind"] = embedder.group(1)
    return result


def physis_diff(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    """Did the change move the way it was asked to? Native `direction` command."""
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("direction"):
        return _unsupported(pool, "direction")

    before = str(merged.get("before") or "").strip()
    after = str(merged.get("after") or "").strip()
    if not before or not after:
        return {"ok": False, "error": "before and after file paths are required"}
    for item in (before, after):
        if not Path(item).expanduser().is_file():
            return {"ok": False, "error": f"file not found: {item}"}
    want = str(merged.get("want") or "shorter").strip().lower()
    if want not in {"shorter", "longer", "rephrased"}:
        return {"ok": False, "error": "want must be shorter, longer or rephrased"}

    result = _run(
        pool,
        [
            "direction",
            "--before", str(Path(before).expanduser()),
            "--after", str(Path(after).expanduser()),
            "--want", want,
            "--json",
        ],
        merged,
    )
    payload = _json_payload(result)
    if payload:
        for key in (
            "direction",
            "before_tokens",
            "after_tokens",
            "moved",
            "retention",
            "null_retention",
            "retention_is_lexical",
            "embedder",
        ):
            if key in payload:
                result[key] = payload[key]
        # `direction` uses a non-zero exit to encode the verdict; the JSON
        # payload is the result, not an engine failure.
        if result.get("returncode") not in (0, None):
            result["ok"] = True
    result["note"] = (
        "`moved` compares the change against the null of doing nothing; "
        "retention_is_lexical=true means this is a lexical, not semantic, measure."
    )
    return result


def physis_chain(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    """Whole-engine pass: structure, context, gaps, drift and the control."""
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("chain"):
        return _unsupported(pool, "chain")

    path = _path(merged)
    if not Path(path).is_dir():
        return {"ok": False, "error": f"corpus directory not found: {path}"}
    try:
        budget = min(4000, max(100, int(merged.get("budget") or 1200)))
    except (TypeError, ValueError):
        budget = 1200
    try:
        threshold = float(merged.get("threshold") or 0.75)
    except (TypeError, ValueError):
        threshold = 0.75

    argv = ["chain", "--corpus", path, "--budget", str(budget), "--threshold", str(threshold), "--json"]
    task = str(merged.get("task") or merged.get("query") or "").strip()
    if task:
        argv += ["--query", task]
    result = _run(pool, argv, merged)
    payload = _json_payload(result)
    if payload:
        result.update(payload)
        # chain uses a non-zero exit to encode its verdict; valid JSON is the
        # result, not an engine failure.
        if result.get("returncode") not in (0, None) and not (result.get("stderr") or "").strip():
            result["ok"] = True
            result["note"] = "non-zero exit encodes the verdict; JSON payload is valid"
    return result


def physis_grid(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    """The adaptive grid: axes and shape from the loaded ontologies."""
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("grid"):
        return _unsupported(pool, "grid")
    result = _run(pool, ["grid", "--json"], merged)
    payload = _json_payload(result)
    if payload:
        for key in (
            "domains",
            "modes",
            "shape",
            "total_cells",
            "populated_cells",
            "entries",
            "expanded",
            "embedder",
        ):
            if key in payload:
                result[key] = payload[key]
        result.pop("stdout", None)
    return result


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
    value = str(merged.get("input") or merged.get("value") or "").strip()
    if not value:
        return {"ok": False, "error": "input is required"}
    tools = str(merged.get("tools") or "").strip()
    if not tools:
        return {"ok": False, "error": "tools registry path is required (tools=...)"}
    if not Path(tools).expanduser().is_file():
        return {"ok": False, "error": f"tools registry not found: {tools}"}
    if not pool.supports("run"):
        return _unsupported(pool, "run")

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
    if not pool.supports("record"):
        return _unsupported(pool, "record")
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
    "physis_chain": physis_chain,
    "physis_grid": physis_grid,
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
            if not pool.supports("scan"):
                return None
            # `scan` is idempotent and registers searchable nodes; `watch`
            # only appends to the observation log, so it would not make the
            # edited file queryable.
            pool.run(
                ["scan", folder],
                cwd=folder,
                timeout=min(int(config.get("timeout") or 20), 8),
            )
        except Exception:
            return None
        return None

    return _hook
