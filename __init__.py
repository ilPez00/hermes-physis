"""Hermes plugin entry point for Physis.

Hermes loads this directory as ``hermes_plugins.physis``. The manifest declares
``provides_tools``, so the loader also imports ``tools.py`` and calls its
``register_tools(ctx)`` for CLI/TUI sessions; this module keeps the same
registration for the package path so both loaders agree.
"""

from __future__ import annotations

from pathlib import Path

try:
    from .tools import (
        make_physis_command,
        make_post_tool_call,
        make_pre_llm_call,
        make_pre_verify,
        register_tools,
    )
except ImportError:  # pragma: no cover - direct import from the plugin directory
    from tools import (
        make_physis_command,
        make_post_tool_call,
        make_pre_llm_call,
        make_pre_verify,
        register_tools,
    )

PLUGIN_DIR = Path(__file__).resolve().parent
SKILL_PATH = PLUGIN_DIR / "skills" / "physis" / "SKILL.md"


def register(ctx) -> None:
    """Register tools, the optional hooks, the /physis command and the skill."""
    register_tools(ctx)

    # Hooks get the context, not a settings snapshot: both re-read per call, so
    # a config edit applies without reloading the plugin.
    hook = getattr(ctx, "register_hook", None)
    if callable(hook):
        for name, factory in (
            ("post_tool_call", make_post_tool_call),
            ("pre_verify", make_pre_verify),
            ("pre_llm_call", make_pre_llm_call),
        ):
            try:
                hook(name, factory(ctx))
            except Exception:
                # A hook is an optimization; never fail plugin load over it.
                pass

    command = getattr(ctx, "register_command", None)
    if callable(command):
        try:
            command(
                "physis",
                make_physis_command(ctx),
                description="Physis status, learned workflow candidates, and model-free recipe replay",
                args_hint="[status|recipes|candidates|replay <prompt>|approve <id>]",
            )
        except Exception:
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
