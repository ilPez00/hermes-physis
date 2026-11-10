"""The registration contract Hermes actually calls."""

import importlib.util
import sys
from pathlib import Path

import pytest

PLUGIN_DIR = Path(__file__).resolve().parents[1]


class FakeContext:
    def __init__(self, settings=None):
        self.settings = settings or {}
        self.tools = {}
        self.hooks = {}
        self.skills = {}
        self.commands = {}

    def get_config(self, key, default=None):
        return self.settings.get(key, default)

    def register_tool(self, name, toolset, schema, handler, **kwargs):
        assert name == schema["name"]
        self.tools[name] = {"toolset": toolset, "handler": handler, "schema": schema}

    def register_hook(self, name, callback):
        self.hooks[name] = callback

    def register_command(self, name, handler, description="", args_hint=""):
        self.commands[name] = handler

    def register_skill(self, name, path, description="", frontmatter=None):
        self.skills[name] = Path(path)


def load_package():
    """Import the plugin the way Hermes does: as a package."""
    name = "hermes_plugins_physis_test"
    if name in sys.modules:
        del sys.modules[name]
    spec = importlib.util.spec_from_file_location(
        name, PLUGIN_DIR / "__init__.py", submodule_search_locations=[str(PLUGIN_DIR)]
    )
    module = importlib.util.module_from_spec(spec)
    module.__package__ = name
    module.__path__ = [str(PLUGIN_DIR)]
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def test_register_registers_all_manifest_tools():
    plugin = load_package()
    ctx = FakeContext()
    plugin.register(ctx)

    from schemas import ALL_NAMES

    assert set(ctx.tools) == set(ALL_NAMES)
    assert all(entry["toolset"] == "physis" for entry in ctx.tools.values())
    assert "post_tool_call" in ctx.hooks
    assert "physis" in ctx.skills


def test_registered_handler_runs(tmp_path):
    plugin = load_package()
    binary = tmp_path / "fake"
    binary.write_text(
        "#!/usr/bin/env python3\n"
        "import sys\n"
        "if sys.argv[1:] == ['--help']:\n"
        "    print('Commands:\\n  snapshot  Show')\n"
        "    raise SystemExit(0)\n"
        "print('Coherence snapshot:\\n  nodes:            7')\n"
    )
    binary.chmod(0o755)
    ctx = FakeContext(settings={"engine_bin": str(binary)})
    plugin.register(ctx)

    import json as _json
    result = ctx.tools["physis_snapshot"]["handler"]({})
    if isinstance(result, str):
        result = _json.loads(result)
    assert result["ok"] is True
    assert result["nodes"] == 7


def test_settings_flow_into_handlers():
    plugin = load_package()
    ctx = FakeContext(settings={"timeout": 7})
    plugin.register(ctx)
    from tools import DEFAULTS

    assert DEFAULTS["timeout"] == 20  # config is resolved per call, not global
    handler = ctx.tools["physis_engine_status"]["handler"]
    result = handler({})
    import json as _json
    if isinstance(result, str):
        result = _json.loads(result)
    assert result["ok"] is True


def test_hook_is_safe_without_engine(tmp_path):
    plugin = load_package()
    ctx = FakeContext(settings={"auto_observe": True, "engine_bin": str(tmp_path / "nope")})
    plugin.register(ctx)
    hook = ctx.hooks["post_tool_call"]
    # ``args`` is what Hermes dispatches (model_tools._emit_post_tool_call_hook);
    # ``tool_args`` is only a compatibility alias.
    assert hook(tool_name="write_file", args={"path": str(tmp_path / "x.py")}) is None
    assert hook(tool_name="write_file", tool_args={"path": str(tmp_path / "x.py")}) is None


def test_register_adds_hooks_and_command():
    plugin = load_package()
    ctx = FakeContext()
    plugin.register(ctx)
    assert set(ctx.hooks) == {"post_tool_call", "pre_verify", "pre_llm_call"}
    assert "physis" in ctx.commands


def test_manifest_matches_registered_tools():
    import yaml

    manifest = yaml.safe_load((PLUGIN_DIR / "plugin.yaml").read_text())
    plugin = load_package()
    ctx = FakeContext()
    plugin.register(ctx)
    assert set(manifest["provides_tools"]) == set(ctx.tools)
    assert "post_tool_call" in manifest["provides_hooks"]
