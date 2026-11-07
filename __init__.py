"""Hermes plugin entry point for Physis.

Hermes loads this directory as ``hermes_plugins.physis``. The manifest declares
``provides_tools``, so the loader also imports ``tools.py`` and calls its
``register_tools(ctx)`` for CLI/TUI sessions; this module keeps the same
registration for the package path so both loaders agree.
"""

from __future__ import annotations

from pathlib import Path

try:
    from .tools import make_post_tool_call, plugin_config, register_tools
except ImportError:  # pragma: no cover - direct import from the plugin directory
    from tools import make_post_tool_call, plugin_config, register_tools

PLUGIN_DIR = Path(__file__).resolve().parent
SKILL_PATH = PLUGIN_DIR / "skills" / "physis" / "SKILL.md"


def register(ctx) -> None:
    """Register tools, the optional incremental-index hook and the skill."""
    config = plugin_config(ctx)
    register_tools(ctx)

    hook = getattr(ctx, "register_hook", None)
    if callable(hook):
        try:
            hook("post_tool_call", make_post_tool_call(config))
        except Exception:
            # A hook is an optimization; never fail plugin load over it.
            pass

    register_skill = getattr(ctx, "register_skill", None)
    if SKILL_PATH.is_file() and callable(register_skill):
        try:
            register_skill(
                "physis",
                SKILL_PATH,
                "Evidence-backed code context with Physis and replant",
            )
        except Exception:
            pass
