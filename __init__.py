"""Hermes plugin entry point."""

from __future__ import annotations

import os
from pathlib import Path

from schemas import ALL_SCHEMAS, ALL_NAMES
from tools import HANDLERS

PLUGIN_DIR = Path(__file__).resolve().parent
SKILL_PATH = PLUGIN_DIR / "skills" / "physis" / "SKILL.md"


def _config(ctx) -> dict:
    cfg = getattr(ctx, "config", None)
    if isinstance(cfg, dict):
        # Support both flat and nested Hermes config layouts.
        for key in ("settings", "physis", "plugins"):
            if isinstance(cfg.get(key), dict):
                cfg = cfg[key]
                break
        return cfg if isinstance(cfg, dict) else {}
    return {}


def _register_tools(ctx, config: dict) -> None:
    for schema in ALL_SCHEMAS:
        name = schema["name"]
        handler = HANDLERS[name]

        def make_handler(n=name):
            def wrapped(args=None, **kwargs):
                merged = dict(config)
                if isinstance(args, dict):
                    merged.update(args)
                merged.update(kwargs)
                return HANDLERS[n](merged)
            return wrapped

        if hasattr(ctx, "register_tool"):
            try:
                ctx.register_tool(name, schema, make_handler())
            except TypeError:
                # Some Hermes versions accept (schema, handler).
                ctx.register_tool(schema, make_handler())


def _post_tool_call(tool_name=None, tool_args=None, result=None, **kwargs):
    try:
        config = kwargs.get("config") or {}
        if not config.get("auto_observe", False):
            return None
        from tools import _engine_from_args, _path

        merged = dict(config)
        merged["timeout"] = min(int(config.get("timeout", 15) or 15), 5)
        path = "."
        if isinstance(tool_args, dict):
            path = tool_args.get("path") or tool_args.get("file_path") or "."
        merged["path"] = path
        engine = _engine_from_args(merged)
        if not engine.supports("observe"):
            return None
        outcome = engine.run(["observe", path, "--max-files", "1000"], cwd=path, timeout=merged["timeout"])
        if outcome.get("ok"):
            return {"physis_observe": "updated"}
        return None
    except Exception:
        return None


def register(ctx) -> None:
    config = _config(ctx)
    # Environment fallback for the engine binary.
    if not config.get("engine_bin"):
        config["engine_bin"] = os.environ.get("PHYSIS_BIN", "")
    # Default to the bundled physis-core if nothing configured.
    if not config.get("engine_bin"):
        bundled = str(PLUGIN_DIR / "bin" / "physis-core")
        if os.path.isfile(bundled) and os.access(bundled, os.X_OK):
            config["engine_bin"] = bundled

    _register_tools(ctx, config)

    if hasattr(ctx, "register_hook"):
        try:
            ctx.register_hook("post_tool_call", _post_tool_call)
        except Exception:
            pass

    if SKILL_PATH.is_file() and hasattr(ctx, "register_skill"):
        try:
            ctx.register_skill("physis", str(SKILL_PATH))
        except Exception:
            pass
