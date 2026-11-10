"""Tool handlers for the Physis Hermes plugin.

The handlers are deliberately thin: they validate arguments, call the engine
command that actually exists, and parse the engine's own output into fields a
model can use. No handler invents evidence the engine did not return.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

try:  # imported as hermes_plugins.<slug>.tools
    from .engine import EngineError, EnginePool, child_env, discover_engines
except ImportError:  # pragma: no cover - direct import from the plugin directory
    from engine import EngineError, EnginePool, child_env, discover_engines

TOOLSET = "physis"

# Fixed plugin-owned profile directory for all replant state (recipes,
# candidates, learned registries, artifact sidecars). It is deliberately
# independent of any repository path a model names: a repo-controlled location
# would let whoever controls the repo plant an executable registry or recipe.
PROFILE_DIR = Path.home() / ".physis"
OPERATOR_STORE_ENV = "PHYSIS_REPLANT_STORE"

DEFAULTS: dict[str, Any] = {
    "engine_bin": "",
    "model_dir": "",
    "semantic": True,
    "timeout": 20,
    "budget": 800,
    "auto_observe": False,
    "auto_expand": True,
    "verify_cmd": "",
    "replant_tools": "",
    "auto_learn": False,
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
_EXPANDED_RE = re.compile(r"Ontology expanded:\s*(.+?)\s*\(([^)]+)\)")
_APPENDED_RE = re.compile(r"appended\s+(\d+)\s+observation")


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
    binary = str(args.get("engine_bin") or "")
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


def _replant_store() -> str:
    """The fixed replant state directory, independent of any model-supplied path.

    Operators (and the test suite) may relocate it with
    ``PHYSIS_REPLANT_STORE``; tool arguments never choose it.
    """
    override = os.environ.get(OPERATOR_STORE_ENV, "").strip()
    if override:
        return str(Path(override).expanduser())
    return str(PROFILE_DIR / "replant")


def _model_env(args: dict[str, Any]) -> dict[str, str] | None:
    model_dir = str(args.get("model_dir") or "").strip()
    semantic = args.get("semantic", True)
    if isinstance(semantic, str):
        semantic = semantic.strip().lower() not in ("0", "false", "no", "off")
    env: dict[str, str] = {}
    if model_dir:
        env["PHYSIS_MODEL_DIR"] = model_dir
        env["PHYSIS_MODEL_DIM"] = "384"
    if not semantic:
        env["PHYSIS_SEMANTIC"] = "0"
    return env or None


def _index_env(args: dict[str, Any]) -> dict[str, str]:
    """One graph store per repository, derived from the resolved path.

    A single global ``~/.physis`` graph mixes nodes from every scanned repo, so
    a query for this task can return evidence from an unrelated project.
    ``PHYSIS_CORE_DIR`` still wins when an operator sets it explicitly.
    """
    if os.environ.get("PHYSIS_CORE_DIR"):
        return {}
    root = _path(args)
    digest = hashlib.sha256(root.encode("utf-8")).hexdigest()[:12]
    return {"PHYSIS_CORE_DIR": str(Path.home() / ".physis" / "repos" / digest)}


def _index_dir(args: dict[str, Any]) -> str:
    return _index_env(args).get("PHYSIS_CORE_DIR") or os.environ.get(
        "PHYSIS_CORE_DIR", str(Path.home() / ".physis")
    )


def _run(
    pool: EnginePool,
    argv: list[str],
    args: dict[str, Any],
    stdin: str | None = None,
    env: dict[str, str] | None = None,
    engine: str | None = None,
) -> dict[str, Any]:
    call_env = _model_env(args)
    call_env = {**(call_env or {}), **_index_env(args)}
    if env:
        call_env = {**call_env, **env}
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


def physis_trace(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    """Port of ReasonTrace's trace diagnostics: evidence coverage, grounding,
    unsupported claims, unexplained actions, repeated calls, loops."""
    import json as _json

    merged = args if isinstance(args, dict) else {}
    raw = merged.get("trace")
    path = merged.get("path")
    if raw in (None, "") and path:
        try:
            with open(str(path), "r", encoding="utf-8") as f:
                raw = f.read()
        except OSError as exc:
            return {"ok": False, "error": f"cannot read trace: {exc}"}
    if raw in (None, ""):
        return {"ok": False, "error": "trace or path is required"}
    if isinstance(raw, str):
        try:
            trace = _json.loads(raw)
        except _json.JSONDecodeError as exc:
            return {"ok": False, "error": f"invalid trace JSON: {exc}"}
    else:
        trace = raw
    if not isinstance(trace, dict):
        return {"ok": False, "error": "trace must be a JSON object"}
    events = trace.get("events")
    if not isinstance(events, list):
        return {"ok": False, "error": "trace.events must be an array"}

    TYPE_BIT = {
        "hypothesis": 1, "action": 2, "observation": 4, "belief_update": 8,
        "failure": 16, "decision": 32, "final_answer": 64, "reasoning": 128,
        "activation": 256, "derivation": 512, "translation": 1024,
    }

    def bit(t):
        return TYPE_BIT.get(str(t), 0)

    by_id = {}
    for e in events:
        if isinstance(e, dict) and "id" in e:
            by_id[e["id"]] = e

    waiting = {}
    children = {}
    for e in events:
        if not isinstance(e, dict):
            continue
        eid = e.get("id")
        known = [s for s in (e.get("supports") or []) if s in by_id]
        waiting[eid] = len(known)
        for sid in known:
            children.setdefault(sid, []).append(eid)
    order = [e.get("id") for e in events if isinstance(e, dict) and waiting.get(e.get("id"), 0) == 0]
    i = 0
    while i < len(order):
        for child in children.get(order[i], []):
            waiting[child] = waiting.get(child, 0) - 1
            if waiting[child] == 0:
                order.append(child)
        i += 1
    if len(order) != len(by_id):
        return {"ok": False, "error": "support graph has a cycle"}

    upstream = {}
    for eid in order:
        e = by_id[eid]
        mask = 0
        for parent in (e.get("supports") or []):
            if parent in by_id:
                mask |= upstream.get(parent, 0) | bit(by_id[parent].get("type"))
        upstream[eid] = mask
    downstream = {eid: 0 for eid in by_id}
    for i in range(len(order) - 1, -1, -1):
        eid = order[i]
        e = by_id[eid]
        below = downstream.get(eid, 0) | bit(e.get("type"))
        for parent in (e.get("supports") or []):
            if parent in by_id:
                downstream[parent] = downstream.get(parent, 0) | below

    def can_reach(eid, types):
        mask = 0
        for t in types:
            mask |= TYPE_BIT.get(t, 0)
        return (downstream.get(eid, 0) & mask) != 0

    def has_ancestor(eid, types):
        mask = 0
        for t in types:
            mask |= TYPE_BIT.get(t, 0)
        return (upstream.get(eid, 0) & mask) != 0

    children_count = {}
    for e in events:
        if isinstance(e, dict):
            for sid in (e.get("supports") or []):
                children_count[sid] = children_count.get(sid, 0) + 1

    counts = {
        "hypothesis": 0, "action": 0, "observation": 0, "failure": 0,
        "belief_update": 0, "decision": 0, "final_answer": 0,
    }
    for e in events:
        if isinstance(e, dict):
            t = e.get("type")
            if t in counts:
                counts[t] += 1

    findings = []
    warnings = []
    def add_finding(rule, fid, severity, category, title, detail, event_ids, recommendation):
        findings.append({
            "rule": rule, "id": fid, "severity": severity, "category": category,
            "title": title, "detail": detail, "eventIds": event_ids,
            "recommendation": recommendation,
        })
        warnings.append(f"{title}: {detail}")

    records_support = any(isinstance(e, dict) and (e.get("supports") or []) for e in events)

    if not records_support:
        add_finding("support-not-recorded", "support-not-recorded", "info", "visibility",
                    "Support links not recorded",
                    "This trace does not say which events support which, so grounding, "
                    "unsupported claims and unexplained actions cannot be checked.",
                    [], "Use a trace format that records which observations each claim relies on.")

    unsupported = []
    if records_support:
        for e in events:
            if isinstance(e, dict) and e.get("type") == "hypothesis":
                eid = e.get("id")
                if not children_count.get(eid) or not can_reach(eid, ["action", "observation", "belief_update", "final_answer"]):
                    unsupported.append(eid)
                    add_finding("unsupported-claim", f"unsupported-{eid}", "warning", "grounding",
                                "Unsupported claim",
                                f"Step {e.get('step')} does not lead to an action, observation, update, or answer.",
                                [eid], "Collect evidence for this claim or mark it as abandoned.")

    unplanned = []
    if records_support:
        for e in events:
            if isinstance(e, dict) and e.get("type") == "action":
                eid = e.get("id")
                if not has_ancestor(eid, ["hypothesis", "decision"]):
                    unplanned.append(eid)
                    add_finding("unplanned-action", f"unplanned-{eid}", "info", "planning",
                                "Unexplained action",
                                f"Step {e.get('step')} has no linked hypothesis or decision. This may be normal for an opaque agent.",
                                [eid], "Link the action to a goal or keep it classified as an observed behavior.")

    final_answers = [e for e in events if isinstance(e, dict) and e.get("type") == "final_answer"]
    ungrounded = [fa for fa in final_answers if not has_ancestor(fa.get("id"), ["observation"])] if records_support else []
    if not final_answers:
        answer_grounding = "no_answer"
    elif not records_support:
        answer_grounding = "not_recorded"
    elif ungrounded:
        answer_grounding = "ungrounded"
    else:
        answer_grounding = "grounded"
    if answer_grounding == "ungrounded":
        add_finding("ungrounded-answer", "ungrounded-answer", "critical", "grounding",
                    "Answer lacks an evidence path",
                    f"{len(ungrounded)} of {len(final_answers)} final answers have no support path back to an observation.",
                    [e.get("id") for e in ungrounded],
                    "Inspect the answer's support links and add the observations that justify it.")

    actions = [e for e in events if isinstance(e, dict) and e.get("type") == "action"]
    def call_key(e):
        return f"{e.get('tool') or ''}\x00{str(e.get('content') or '').strip().lower()}"
    identical = {}
    for a in actions:
        identical.setdefault(call_key(a), []).append(a)
    for group in identical.values():
        if len(group) < 2:
            continue
        add_finding("same-call", f"same-call-{group[0].get('id')}", "warning", "loop",
                    "Same call repeated",
                    f"The same {group[0].get('tool') and (str(group[0].get('tool')) + ' call') or 'action'} "
                    f"appears {len(group)} times, at steps {', '.join(str(e.get('step')) for e in group)}.",
                    [e.get("id") for e in group],
                    "Check whether anything changed between the calls that could give a different result.")

    run = []
    def close_run():
        nonlocal run
        if len(run) >= 2 and len(set(call_key(e) for e in run)) > 1:
            add_finding("tool-run", f"tool-run-{run[0].get('id')}", "warning", "loop",
                        "Repeated tool call",
                        f"{run[0].get('tool')} is called {len(run)} times in a row, at steps "
                        f"{', '.join(str(e.get('step')) for e in run)}, with no other action between.",
                        [e.get("id") for e in run],
                        "Check whether each call used what the previous one returned, or repeated a step that was not working.")
        run = []
    for a in actions:
        if not a.get("tool") or (run and run[0].get("tool") != a.get("tool")):
            close_run()
        if a.get("tool"):
            run.append(a)
    close_run()

    for e in events:
        if isinstance(e, dict) and e.get("type") == "belief_update" and e.get("label") == "stated" and e.get("confidence") is None:
            add_finding("missing-confidence", f"missing-confidence-{e.get('id')}", "info", "confidence",
                        "Uncalibrated update",
                        f"Step {e.get('step')} changes a belief without a confidence value.",
                        [e.get("id")],
                        "Record confidence when the runtime provides it; do not invent it after the fact.")

    for e in events:
        if isinstance(e, dict) and e.get("type") == "failure":
            add_finding("failure-recorded", f"failure-{e.get('id')}", "critical", "failure",
                        "Failure recorded",
                        f"A failure event appears at step {e.get('step')}.",
                        [e.get("id")],
                        "Trace the incoming evidence and the next recovery action.")

    claims = [e for e in events if isinstance(e, dict) and e.get("type") in ("hypothesis", "belief_update", "decision", "final_answer")]
    supported_claims = [e for e in claims if has_ancestor(e.get("id"), ["observation"])] if records_support else []
    evidence_coverage = round((len(supported_claims) / len(claims)) * 100) if (records_support and claims) else None

    opaque_count = sum(1 for e in events if isinstance(e, dict) and e.get("label") == "observed")
    inferred_count = sum(1 for e in events if isinstance(e, dict) and e.get("label") == "inferred")
    if opaque_count:
        add_finding("observed-events", "opaque-trace", "info", "visibility",
                    "Observed behavior trace",
                    f"{opaque_count} events are observable outputs or tool records, not private model reasoning.",
                    [e.get("id") for e in events if isinstance(e, dict) and e.get("label") == "observed"],
                    "Use this trace to evaluate behavior and evidence paths; do not infer hidden thoughts from it.")
    if inferred_count:
        add_finding("inferred-events", "inferred-events", "info", "visibility",
                    "Analyst inferences present",
                    f"{inferred_count} events were reconstructed from behavior.",
                    [e.get("id") for e in events if isinstance(e, dict) and e.get("label") == "inferred"],
                    "Treat inferred nodes as hypotheses about behavior, not observations.")

    return {
        "ok": True,
        "trace_title": trace.get("title"),
        "event_count": len(events),
        "counts": counts,
        "hypothesis_to_observation_ratio": (
            round(counts["hypothesis"] / counts["observation"], 2) if counts["observation"] else None
        ),
        "unsupported_hypotheses": unsupported,
        "actions_without_prior_hypothesis": unplanned,
        "answer_grounding": answer_grounding,
        "grounded_answer_count": len(final_answers) - len(ungrounded),
        "evidence_coverage": evidence_coverage,
        "opaque_event_count": opaque_count,
        "inferred_event_count": inferred_count,
        "findings": findings,
        "warnings": warnings,
    }


def physis_engine_status(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    info = pool.info()
    info["ok"] = True
    info["store"] = _index_dir(merged)
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

    task = str(merged.get("task") or "").strip()
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

    question = str(merged.get("question") or "").strip()
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

    obj = str(merged.get("object") or "").strip()
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

    argv = ["classify", "--json", text]
    if merged.get("auto_expand") is False:
        argv.append("--no-expand")
    result = _run(pool, argv, merged)
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
    stdout = (result.get("stdout") or "").strip()
    if stdout.startswith("{"):
        try:
            payload = json.loads(stdout)
        except (ValueError, TypeError):
            payload = None
        if isinstance(payload, dict):
            result["classification"] = payload
            result["cells"] = payload.get("top", [])
            result["margin"] = payload.get("margin")
            result["confidence"] = payload.get("confidence")
            result["best_entry"] = payload.get("best_entry")
            result["best_domain"] = payload.get("best_domain")
            result["best_mode"] = payload.get("best_mode")
            result["grid_shape"] = (payload.get("grid") or {}).get("shape")
            result["grid_entries"] = (payload.get("grid") or {}).get("entries")
            if payload.get("created_entry"):
                result["created_entry"] = payload["created_entry"]
                result["created_cell"] = payload.get("created_cell")
            embedder = (payload.get("embedder") or {}).get("kind")
            if embedder:
                result["embedder_kind"] = embedder
            return result

    embedder = _EMBEDDER_LINE_RE.search(stdout)
    if embedder:
        result["embedder_kind"] = embedder.group(1)
    expanded = _EXPANDED_RE.search(stdout)
    if expanded:
        result["created_entry"] = expanded.group(1).strip()
        result["created_cell"] = expanded.group(2).strip()
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


def physis_ingest(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("watch"):
        return _unsupported(pool, "watch")
    source = str(merged.get("source") or "fs").strip().lower()
    if source not in {"fs", "git", "terminal", "agent", "process", "browser", "all"}:
        return {"ok": False, "error": "source must be fs, git, terminal, agent, process, browser or all"}
    try:
        limit = max(1, int(merged.get("max") or 500))
    except (TypeError, ValueError):
        limit = 500
    argv = ["watch", "--source", source, "--max", str(limit)]
    if source in {"fs", "git"}:
        path = _path(merged)
        if not Path(path).exists():
            return {"ok": False, "error": f"path not found: {path}"}
        argv += ["--path", path]
    if merged.get("dry_run"):
        argv.append("--dry-run")
    result = _run(pool, argv, merged)
    match = _APPENDED_RE.search(result.get("stdout") or "")
    if match:
        result["appended"] = int(match.group(1))
    result["source"] = source
    return result


def physis_observed(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("observed"):
        return _unsupported(pool, "observed")
    try:
        limit = max(1, int(merged.get("limit") or 20))
    except (TypeError, ValueError):
        limit = 20
    argv = ["observed", "--json", "--limit", str(limit)]
    source = str(merged.get("source") or "").strip()
    if source:
        argv += ["--source", source]
    concurrent = str(merged.get("concurrent") or "").strip()
    if concurrent:
        argv += ["--concurrent", concurrent]
    result = _run(pool, argv, merged)
    text = (result.get("stdout") or "").strip()
    if text.startswith("["):
        try:
            result["observations"] = json.loads(text)
            result["count"] = len(result["observations"])
        except (ValueError, TypeError):
            pass
    return result


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


# --------------------------------------------------------------------------- #
# Artifact freshness: a verified recipe is only reusable while its inputs are
# the ones the checks passed against. replant itself fingerprints the *tool*
# implementation (program + declared deps); these sidecars cover the task inputs.
# --------------------------------------------------------------------------- #

MAX_ARTIFACT_BYTES = 1 << 20


def _request_digest(args: dict[str, Any]) -> str:
    """Key the sidecar by the same request the engine is sent."""
    request = {
        "prompt": str(args.get("input") or "").strip(),
        "input": {},
        "context": {},
    }
    return hashlib.sha256(json.dumps(request, sort_keys=True).encode("utf-8")).hexdigest()


def _artifact_hashes(paths: list[Any]) -> dict[str, str]:
    hashes: dict[str, str] = {}
    for raw in paths:
        path = Path(str(raw)).expanduser()
        try:
            if not path.is_file() or path.stat().st_size > MAX_ARTIFACT_BYTES:
                raise ValueError(f"artifact unavailable or too large: {path}")
            data = path.read_bytes()
            if len(data) > MAX_ARTIFACT_BYTES:
                raise ValueError(f"artifact too large: {path}")
            hashes[str(path)] = hashlib.sha256(data).hexdigest()
        except OSError as exc:
            raise ValueError(f"artifact unavailable: {path}") from exc
    return hashes


def _artifacts_file(args: dict[str, Any]) -> Path:
    return Path(_replant_store()) / "artifacts" / f"{_request_digest(args)}.json"


def _private_store() -> Path:
    """replant refuses a state dir other users can read; create it the same way."""
    store = Path(_replant_store())
    store.mkdir(parents=True, exist_ok=True, mode=0o700)
    try:
        os.chmod(store, 0o700)
    except OSError:
        pass
    return store


def _write_artifacts(args: dict[str, Any], artifacts: list[Any]) -> None:
    target = _artifacts_file(args)
    _private_store()
    target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    target.write_text(json.dumps({"artifacts": _artifact_hashes(artifacts)}, indent=2))


def _changed_artifacts(args: dict[str, Any]) -> list[str]:
    """Inputs recorded at verification time whose content changed since."""
    target = _artifacts_file(args)
    try:
        payload = json.loads(target.read_text())
        recorded = payload["artifacts"]
        if not isinstance(recorded, dict) or any(
            not isinstance(path, str) or not isinstance(digest, str)
            or not re.fullmatch(r"[0-9a-f]{64}", digest)
            for path, digest in recorded.items()
        ):
            raise ValueError("invalid fingerprints")
    except (OSError, ValueError, TypeError, KeyError, AttributeError) as exc:
        raise ValueError("freshness evidence missing or invalid") from exc
    changed = []
    for path, digest in recorded.items():
        try:
            current = _artifact_hashes([path])[path]
        except ValueError:
            changed.append(path)
            continue
        if current != digest:
            changed.append(path)
    return sorted(changed)


# --------------------------------------------------------------------------- #
# Learning: a session that passed a real check becomes a reviewable candidate
# --------------------------------------------------------------------------- #

MAX_LEARN_STEPS = 12
MAX_LEARN_FILES = 16
MAX_LEARN_SESSIONS = 32
MAX_LEARN_COMMAND = 2000

_LEARN: dict[str, dict[str, Any]] = {}

STEP_WRAPPER = '''#!/usr/bin/env python3
"""Replay-step wrapper: run one recorded command, report one JSON object."""
import json, os, subprocess, sys

command = sys.argv[1]
cwd = sys.argv[2] or None
# Fixed minimal child environment: only what a recorded shell step plausibly
# needs. Nothing else is inherited, so no stray PHYSIS_* or secrets reach it.
env = {k: os.environ[k] for k in ("PATH", "HOME", "TMPDIR", "LANG", "LC_ALL", "TZ")
       if k in os.environ}
proc = subprocess.run(
    command, shell=True, cwd=cwd, capture_output=True, text=True,
    stdin=subprocess.DEVNULL, env=env,
)
stdout = ((proc.stdout or "").strip())[-2000:]
stderr = ((proc.stderr or "").strip())[-2000:]
output = ((proc.stdout or "") + (proc.stderr or "")).strip()[-2000:]
print(json.dumps({"ok": proc.returncode == 0, "exit": proc.returncode,
                  "output": output, "stdout": stdout, "stderr": stderr}))
'''


def _learn_state(session: str) -> dict[str, Any]:
    state = _LEARN.get(session)
    if state is None:
        if len(_LEARN) >= MAX_LEARN_SESSIONS:
            _LEARN.pop(next(iter(_LEARN)))
        state = {"user": "", "steps": [], "files": []}
        _LEARN[session] = state
    return state


def reset_learned() -> None:
    """Drop the in-memory traces (tests)."""
    _LEARN.clear()


def _learn_observe(session: str, tool_name: str, call_args: dict[str, Any]) -> None:
    """Remember only what is replayable: shell steps and edited files."""
    state = _learn_state(session)
    if tool_name == "terminal":
        command = str(call_args.get("command") or "").strip()
        if not command or call_args.get("background") or len(command) > MAX_LEARN_COMMAND:
            return
        workdir = str(call_args.get("workdir") or "").strip()
        step = {"command": command, "workdir": workdir}
        if step not in state["steps"] and len(state["steps"]) < MAX_LEARN_STEPS:
            state["steps"].append(step)
    elif tool_name in {"write_file", "patch", "edit_file", "apply_patch"}:
        target = str(call_args.get("path") or call_args.get("file_path") or "").strip()
        if target and target not in state["files"] and len(state["files"]) < MAX_LEARN_FILES:
            path = Path(target).expanduser()
            if path.is_file() and path.stat().st_size <= MAX_ARTIFACT_BYTES:
                state["files"].append(str(path.resolve()))


def _candidates_dir() -> Path:
    return Path(_replant_store()) / "candidates"


def _write_candidate(state: dict[str, Any], check: str) -> dict[str, Any] | None:
    """Persist a candidate for operator approval; never replay it automatically.

    Candidates live in the fixed profile directory, never in the repository:
    a repo-controlled location would let a model plant a candidate whose steps
    are shell commands and then social-engineer (or script) its approval.
    """
    steps = list(state.get("steps") or [])
    if not steps or not check.strip():
        return None
    prompt = str(state.get("user") or "").strip()[:MAX_LEARN_COMMAND]
    if not prompt:
        return None  # no intent recorded: a recipe without a request cannot be matched
    content = {
        "prompt": prompt,
        "steps": steps,
        "files": list(state.get("files") or []),
        "check": check,
    }
    identity = hashlib.sha256(
        json.dumps(content, sort_keys=True).encode("utf-8")
    ).hexdigest()
    candidate = {
        "id": identity,
        **content,
        "created": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    }
    target = _candidates_dir() / f"{identity}.json"
    _private_store()
    target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    target.write_text(json.dumps(candidate, indent=2))
    return candidate


def _load_candidate(candidate_id: str) -> dict[str, Any] | None:
    """Load a candidate from the profile directory only.

    Never search the working tree: a repo-planted candidate must not be
    approvable, whatever id it claims.
    """
    identity = str(candidate_id).strip().lower().lstrip("#")
    if not re.fullmatch(r"[0-9a-f]{64}", identity):
        return None
    path = _candidates_dir() / f"{identity}.json"
    try:
        if path.is_symlink() or not path.is_file():
            return None
        candidate = json.loads(path.read_text())
        content = {key: candidate[key] for key in ("prompt", "steps", "files", "check")}
        digest = hashlib.sha256(json.dumps(content, sort_keys=True).encode("utf-8")).hexdigest()
        if candidate.get("id") == identity == digest and content["steps"]:
            return candidate
    except (OSError, ValueError, TypeError, KeyError, AttributeError):
        return None
    return None


def _learned_registry(candidate: dict[str, Any]) -> Path:
    """Generate the operator-reviewed registry that replant needs.

    Every step is the bundled Python running a fixed wrapper, so the model never
    names an executable: it can only name ``stepN`` / ``verify``.
    """
    learned = _private_store() / "learned"
    learned.mkdir(parents=True, exist_ok=True, mode=0o700)
    wrapper = learned / "step.py"
    wrapper.write_text(STEP_WRAPPER)
    os.chmod(wrapper, 0o700)

    def spec(script: str, cwd: str) -> dict[str, Any]:
        # Run the wrapper directly so replant fingerprints only this small
        # file. Invoking it through a (possibly huge) interpreter would make
        # the interpreter binary part of the fingerprint and trip replant's
        # 64 MiB per-file limit on hosts whose python is larger than that.
        return {
            "kind": "action",
            "program": str(wrapper),
            "args": [script, cwd],
            "fingerprint_files": [str(wrapper)],
            "timeout_ms": 120_000,
        }

    tools = {f"step{index}": spec(str(step["command"]), str(step.get("workdir") or ""))
             for index, step in enumerate(candidate["steps"], 1)}
    first = str((candidate.get("files") or [str(Path.cwd())])[0])
    check_cwd = str(Path(first).parent) if Path(first).is_file() else str(Path.cwd())
    verify = spec(str(candidate["check"]), check_cwd)
    verify["kind"] = "check"
    tools["verify"] = verify

    registry = learned / "tools.json"
    registry.write_text(json.dumps(tools, indent=2))
    return registry


def _file_digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _approval_file() -> Path:
    return _private_store() / "learned" / "approval.json"


def _write_approval(candidate: dict[str, Any], registry: Path) -> None:
    """Bind replay to the exact registry produced during operator approval."""
    payload = {
        "candidate_id": candidate["id"],
        "registry": str(registry),
        "registry_digest": _file_digest(registry),
        "candidate": {
            key: candidate[key] for key in ("prompt", "steps", "files", "check")
        },
    }
    target = _approval_file()
    target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    target.write_text(json.dumps(payload, indent=2, sort_keys=True))


def _verify_learned_approval(registry: Path) -> str | None:
    """Return refusal reason; mutable registry/state is never trusted silently."""
    target = _approval_file()
    try:
        approval = json.loads(target.read_text())
        if approval.get("registry") != str(registry):
            raise ValueError("registry path differs from operator approval")
        if approval.get("registry_digest") != _file_digest(registry):
            raise ValueError("registry content differs from operator approval")
        candidate = approval["candidate"]
        expected = hashlib.sha256(json.dumps(candidate, sort_keys=True).encode("utf-8")).hexdigest()
        if approval.get("candidate_id") != expected:
            raise ValueError("approved candidate binding is invalid")
        if not candidate.get("steps") or not candidate.get("check"):
            raise ValueError("approved executable steps are missing")
    except (OSError, ValueError, TypeError, KeyError, AttributeError):
        return "replay refused: registry or executable steps lack valid operator approval"
    return None


def physis_candidates(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    directory = _candidates_dir()
    if not directory.is_dir():
        return {
            "ok": True,
            "count": 0,
            "candidates": [],
            "hint": "no candidate yet: enable auto_learn and set verify_cmd, then let one turn pass its check",
        }
    candidates = []
    for path in sorted(directory.glob("*.json"), key=lambda item: item.stat().st_mtime, reverse=True):
        try:
            candidate = json.loads(path.read_text())
        except (OSError, ValueError, TypeError):
            continue
        if not isinstance(candidate, dict):
            continue
        candidates.append(
            {
                key: candidate.get(key)
                for key in ("id", "prompt", "steps", "files", "check", "created")
            }
        )
    return {"ok": True, "count": len(candidates), "candidates": candidates}



def _reject_model_supplied(merged: dict[str, Any]) -> dict[str, Any] | None:
    """Registry, store and candidates are never model-supplied.

    Even on direct handler calls (hooks, tests): the executable surface comes
    from operator config (``replant_tools``) or the plugin-generated learned
    registry in the fixed profile directory, and approval is exclusively
    ``/physis approve <id>``.
    """
    forbidden = sorted({"tools", "store", "candidate"} & set(merged))
    if forbidden:
        return {
            "ok": False,
            "error": (
                f"model-supplied {', '.join(forbidden)} rejected: the tools registry "
                "and replant state come from operator config (replant_tools) or the "
                "plugin profile directory, not from tool arguments; approving a learned "
                "workflow is exclusively '/physis approve <id>'"
            ),
        }
    return None


def _replay_registry(merged: dict[str, Any]) -> str:
    """Operator-configured registry, else the plugin-generated learned one."""
    registry = str(merged.get("replant_tools") or "").strip()
    if registry:
        return registry
    learned = Path(_replant_store()) / "learned" / "tools.json"
    return str(learned) if learned.is_file() else ""


def replant_run(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    rejected = _reject_model_supplied(merged)
    if rejected:
        return rejected
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    value = str(merged.get("input") or "").strip()
    if not value:
        return {"ok": False, "error": "input is required"}
    configured_registry = str(merged.get("replant_tools") or "").strip()
    tools = _replay_registry(merged)
    if not tools:
        return {
            "ok": False,
            "error": (
                "no tools registry: set the operator config replant_tools, or approve "
                "a learned workflow first (/physis approve <id>)"
            ),
            "hint": "the registry is operator-owned; it is never taken from a repository path",
        }
    if not Path(tools).expanduser().is_file():
        return {"ok": False, "error": f"tools registry not found: {tools}"}
    if not configured_registry:
        approval_error = _verify_learned_approval(Path(tools).expanduser())
        if approval_error:
            return {
                "ok": False,
                "blocked": True,
                "operation": "lookup",
                "status": "blocked",
                "provider_calls": 0,
                "reason": approval_error,
            }

    try:
        changed = _changed_artifacts(merged)
    except ValueError as exc:
        return {"ok": False, "blocked": True, "operation": "lookup", "status": "blocked",
                "provider_calls": 0, "reason": str(exc)}
    if changed:
        # Refuse rather than replay under preconditions the checks never saw.
        return {
            "ok": False,
            "blocked": True,
            "operation": "lookup",
            "status": "blocked",
            "provider_calls": 0,
            "reason": "task inputs changed since verification; repeat and request operator approval",
            "changed": changed,
        }
    if not pool.supports("run"):
        return _unsupported(pool, "run")

    stdin = json.dumps({"prompt": value, "input": {}, "context": {}})
    result = _run(
        pool,
        ["run", "--tools", str(Path(tools).expanduser()), "--store", _replant_store()],
        merged,
        stdin=stdin,
        engine="replant",
    )
    _copy_keys(result, _json_payload(result))
    return result


def _record_recipe(
    pool: EnginePool,
    merged: dict[str, Any],
    value: str,
    recipe: Any,
    tools: str,
    trace: Any,
    candidate_id: str | None,
) -> dict[str, Any]:
    if not pool.supports("record"):
        return _unsupported(pool, "record")
    # Invalidate old evidence before attempting a replacement, including on failure.
    try:
        _artifacts_file(merged).unlink(missing_ok=True)
    except OSError as exc:
        return {"ok": False, "error": f"cannot invalidate old freshness evidence: {exc}"}
    stdin = json.dumps(
        {
            "request": {"prompt": value, "input": {}, "context": {}},
            "recipe": recipe,
            "trace": trace,
        }
    )
    result = _run(
        pool,
        ["record", "--tools", str(Path(tools).expanduser()), "--store", _replant_store()],
        merged,
        stdin=stdin,
        engine="replant",
    )
    payload = _json_payload(result)
    _copy_keys(result, payload)
    if payload and payload.get("status") == "succeeded":
        # Only a verified record earns a freshness sidecar.
        artifacts = merged.get("artifacts") or []
        if isinstance(artifacts, str):
            artifacts = [item.strip() for item in artifacts.split(",") if item.strip()]
        try:
            _write_artifacts(merged, list(artifacts))
        except (OSError, ValueError) as exc:
            result.update(ok=False, blocked=True, reason=f"could not persist artifact fingerprints: {exc}")
    if candidate_id:
        result["candidate_id"] = candidate_id
    return result


def replant_record(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    """Record an ad-hoc recipe against the operator's registry.

    A model may compose steps out of the tools the operator declared in
    ``replant_tools``; it can never name a registry, a store or a candidate.
    Learned-workflow approval is exclusively ``/physis approve <id>``.
    """
    merged = _with_defaults(_merge(args, kwargs))
    rejected = _reject_model_supplied(merged)
    if rejected:
        return rejected
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)

    value = str(merged.get("input") or "").strip()
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
    tools = str(merged.get("replant_tools") or "").strip()
    if not tools:
        return {
            "ok": False,
            "error": (
                "recording requires the operator's replant_tools registry; approving a "
                "learned workflow is exclusively '/physis approve <id>'"
            ),
        }
    if not Path(tools).expanduser().is_file():
        return {"ok": False, "error": f"tools registry not found: {tools}"}

    trace = merged.get("trace") or {"provider": "hermes-physis-plugin"}
    if isinstance(trace, str):
        try:
            trace = json.loads(trace)
        except (ValueError, TypeError):
            return {"ok": False, "error": "trace must be valid JSON"}
    return _record_recipe(pool, merged, value, recipe, tools, trace, None)


def _approve_candidate(config: dict[str, Any], candidate_id: str) -> dict[str, Any]:
    """Operator-only path behind ``/physis approve <id>``.

    Not reachable through any registered model tool: the candidate carries
    shell steps that will run again unattended, so the decision to record it
    must be the operator's, made from the candidate listing.
    """
    merged = _with_defaults(dict(config))
    candidate = _load_candidate(str(candidate_id).strip())
    if candidate is None:
        return {
            "ok": False,
            "error": f"candidate not found: {candidate_id}",
            "hint": "run physis_candidates",
        }
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    merged["input"] = candidate["prompt"]
    merged["artifacts"] = list(candidate.get("files") or [])
    recipe = {
        "steps": [
            {"tool": f"step{index}", "input": {}}
            for index in range(1, len(candidate["steps"]) + 1)
        ],
        "checks": [{"tool": "verify", "input": {}}],
    }
    result = _record_recipe(
        pool,
        merged,
        candidate["prompt"],
        recipe,
        str(_learned_registry(candidate)),
        {"provider": "hermes-physis-plugin", "approved_by": "operator"},
        candidate["id"],
    )
    if result.get("ok") and result.get("status") == "succeeded":
        try:
            _write_approval(candidate, _learned_registry(candidate))
        except (OSError, ValueError) as exc:
            return {"ok": False, "blocked": True, "reason": f"could not persist replay approval: {exc}"}
    return result


def replant_stats(args: Any = None, **kwargs: Any) -> dict[str, Any]:
    merged = _with_defaults(_merge(args, kwargs))
    try:
        pool = _pool_from_args(merged)
    except EngineError as exc:
        return _engine_error(exc)
    if not pool.supports("stats"):
        return _unsupported(pool, "stats")
    result = _run(pool, ["stats", "--store", _replant_store()], merged, stdin="{}", engine="replant")
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

    # Hermetic demo: a throwaway store and a registry the plugin itself writes
    # from the bundled fixture. Nothing is read from a repository path.
    try:
        with tempfile.TemporaryDirectory(prefix="physis-replant-demo-") as tmp:
            return _demo_pass(pool, merged, Path(tmp))
    except OSError as exc:
        return {"ok": False, "error": str(exc)}


def _demo_pass(pool: EnginePool, merged: dict[str, Any], store: Path) -> dict[str, Any]:
    registry, fixture_root = _write_demo_registry(store)

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
    value = str(merged.get("input") or "").strip()
    if not value:
        return {"ok": False, "error": "input is required"}
    stdin = json.dumps({"prompt": value, "input": {}, "context": {}})
    result = _run(pool, ["show", "--store", _replant_store()], merged, stdin=stdin, engine="replant")
    payload = _json_payload(result)
    if payload:
        result.update(payload)
    return result


HANDLERS = {
    "physis_engine_status": physis_engine_status,
    "physis_trace": physis_trace,
    "physis_observe": physis_observe,
    "physis_snapshot": physis_snapshot,
    "physis_context": physis_context,
    "physis_query": physis_query,
    "physis_why": physis_why,
    "physis_diff": physis_diff,
    "physis_classify": physis_classify,
    "physis_chain": physis_chain,
    "physis_grid": physis_grid,
    "physis_ingest": physis_ingest,
    "physis_observed": physis_observed,
    "replant_run": replant_run,
    "replant_stats": replant_stats,
    "replant_demo": replant_demo,
    "replant_show": replant_show,
    "physis_candidates": physis_candidates,
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
        declared = set((schema.get("parameters") or {}).get("properties") or {})

        def wrapped(args=None, _handler=handler, _ctx=ctx, _name=name, _declared=declared, **kwargs):
            # Read the live plugin config on every call so a config edit applies
            # without reloading the plugin. The model only ever gets to pass
            # arguments the schema declares: executable settings (engine_bin,
            # replant_tools, verify_cmd, ...) come from operator config alone,
            # so naming them as tool arguments is rejected here.
            user = _merge(args, kwargs)
            undeclared = sorted(set(user) - _declared)
            if undeclared:
                return json.dumps(
                    {
                        "ok": False,
                        "error": (
                            f"undeclared argument(s) rejected: {', '.join(undeclared)}"
                        ),
                        "tool": _name,
                        "hint": "only the arguments declared in the tool schema are accepted",
                    }
                )
            config = plugin_config(_ctx)
            merged = dict(config)
            for key, value in user.items():
                if value not in (None, ""):
                    merged[key] = value
                else:
                    merged.setdefault(key, config.get(key, DEFAULTS.get(key)))
            merged = _with_defaults(merged)
            result = _handler(merged)
            if isinstance(result, str):
                return result
            try:
                return json.dumps(result, default=str)
            except (TypeError, ValueError):
                return str(result)

        ctx.register_tool(
            name,
            TOOLSET,
            schema,
            wrapped,
            description=schema.get("description", ""),
        )
        registered.append(name)
    return registered


def _live_config(config: Any) -> dict[str, Any]:
    """Settings for a hook: re-read per call when given the PluginContext.

    A config snapshot taken at load time silently ignores a setting the
    operator changes afterwards; tools.py already re-reads per call.
    """
    if hasattr(config, "get_config"):
        return plugin_config(config)
    return dict(config or DEFAULTS)


def make_post_tool_call(config: Any):
    """Observe real work: bounded re-index, and (opt-in) learn replayable steps."""
    def _hook(
        tool_name=None,
        args=None,
        tool_args=None,
        result=None,
        status="ok",
        session_id="",
        **kwargs,
    ):
        live = _live_config(config)
        # Hermes dispatches ``args``; ``tool_args`` is kept as an alias so a
        # caller using the older spelling still works.
        call_args = args if isinstance(args, dict) else tool_args
        try:
            if (
                live.get("auto_learn")
                and isinstance(call_args, dict)
                and str(status or "ok") != "error"
            ):
                _learn_observe(str(session_id or "default"), str(tool_name or ""), call_args)
        except Exception:
            pass

        try:
            if not live.get("auto_observe"):
                return None
            if tool_name not in {"write_file", "patch", "edit_file", "apply_patch"}:
                return None
            if not isinstance(call_args, dict):
                return None
            target = call_args.get("path") or call_args.get("file_path")
            if not target:
                return None
            folder = str(Path(target).expanduser().resolve().parent)
            timeout = min(int(live.get("timeout") or 20), 8)
            pool = _pool_from_args(
                {"engine_bin": live.get("engine_bin", ""), "timeout": timeout}
            )
            if not pool.supports("scan"):
                return None
            # `scan` is idempotent and registers searchable nodes; `watch`
            # only appends to the observation log, so it would not make the
            # edited file queryable.
            pool.run(
                ["scan", folder],
                cwd=folder,
                timeout=timeout,
                env=_index_env({"path": folder}),
            )
        except Exception:
            return None
        return None

    return _hook


def make_pre_llm_call(config: Any):
    """Remember the request, so a verified session knows what it was asked for."""

    def _hook(session_id="", user_message="", **kwargs):
        try:
            if not _live_config(config).get("auto_learn"):
                return None
            if isinstance(user_message, str) and user_message.strip():
                _learn_state(str(session_id or "default"))["user"] = user_message.strip()[
                    :MAX_LEARN_COMMAND
                ]
        except Exception:
            pass
        return None

    return _hook


def make_pre_verify(config: Any):
    """Run the operator's own verification command after code edits.

    Hermes calls ``pre_verify`` once per turn when the agent edited files and is
    about to finish. A failing command keeps the turn going with the tail of the
    real output, so the deterministic check - not another model - decides
    whether the work is done. A passing one is also the moment a session earns
    the right to become a reusable recipe candidate.
    """
    def _hook(changed_paths=None, attempt=0, coding=False, session_id="", **kwargs):
        try:
            # One nudge per turn: attempt > 0 means the operator/message loop
            # already got the failure and the turn is allowed to finish.
            if attempt:
                return None
            live = _live_config(config)
            command = str(live.get("verify_cmd") or "").strip()
            if not command:
                return None
            paths = [str(item) for item in (changed_paths or []) if str(item).strip()]
            if not paths:
                return None
            cwd = str(Path(paths[0]).expanduser().resolve().parent)
            proc = subprocess.run(
                command,
                shell=True,
                cwd=cwd,
                capture_output=True,
                text=True,
                timeout=min(int(live.get("timeout") or 20), 300),
                check=False,
                env=child_env(),
            )
            if proc.returncode != 0:
                tail = ((proc.stdout or "") + (proc.stderr or "")).strip().splitlines()[-15:]
                return {
                    "action": "continue",
                    "message": (
                        f"Physis verify failed (exit {proc.returncode}): {command}\n"
                        + "\n".join(tail)
                    ),
                }
            if live.get("auto_learn"):
                state = _learn_state(str(session_id or "default"))
                try:
                    _write_candidate(state, command)
                except Exception:
                    pass  # learning is best-effort, never a turn failure
            return None
        except Exception:
            return None

    return _hook


_COMMAND_TOOLS: dict[str, str] = {
    "status": "physis_engine_status",
    "recipes": "replant_stats",
    "replay": "replant_run",
    "candidates": "physis_candidates",
}


def make_physis_command(config: Any):
    """``/physis [status|recipes|candidates|replay|approve]`` - no model call to repeat work.

    ``approve`` is deliberately not a registered model tool: it turns shell
    steps a session recorded into a replayable recipe, so it must stay an
    operator action.
    """

    def _handler(raw_args: str = "", **kwargs):
        live = _live_config(config)
        parts = (raw_args or "").split(maxsplit=1)
        verb = (parts[0] if parts else "status").lower()
        rest = parts[1].strip() if len(parts) > 1 else ""
        if verb not in _COMMAND_TOOLS and verb != "approve":
            return f"Usage: /physis {'|'.join([*_COMMAND_TOOLS, 'approve'])} [prompt|id]"
        payload: dict[str, Any] = dict(live)
        payload["path"] = str(Path.cwd())
        if verb == "replay":
            if not rest:
                return "Usage: /physis replay <prompt>"
            # The registry comes from operator config or the plugin-generated
            # learned registry in the profile dir - never from a repo path.
            payload["input"] = rest
        if verb == "approve":
            if not rest:
                return "Usage: /physis approve <candidate-id>"
            result = _approve_candidate(live, rest)
        else:
            result = HANDLERS[_COMMAND_TOOLS[verb]](payload)
        if verb == "replay":
            result = {
                key: result.get(key)
                for key in ("ok", "operation", "status", "provider_calls", "reason", "blocked", "changed", "error", "hint")
                if key in result
            }
        if verb == "approve":
            result = {
                key: result.get(key)
                for key in ("ok", "operation", "status", "provider_calls", "reason", "candidate_id", "error", "hint")
                if key in result
            }
        return json.dumps(result, default=str)

    return _handler
