import json
import stat
import textwrap
from pathlib import Path

import pytest

from tools import HANDLERS, reset_pool
import tools

CORE_COMMANDS = {
    "scan",
    "search",
    "node-search",
    "snapshot",
    "classify",
    "context",
    "direction",
    "chain",
    "grid",
    "watch",
    "observed",
    "assert",
    "facet",
}


def make_binary(tmp_path: Path, supported: set[str]) -> str:
    help_lines = "Commands:\n" + "".join(f"  {cmd}  description\n" for cmd in sorted(supported))
    path = tmp_path / "fake-engine"
    script = textwrap.dedent(f"""\
    #!/usr/bin/env python3
    import sys
    argv = sys.argv[1:]
    if argv and argv[0] == "--help":
        print({help_lines!r})
        raise SystemExit(0)
    if argv and argv[0] == "--version":
        print("fake-engine 9.9")
        raise SystemExit(0)
    if argv and argv[0] == "snapshot":
        print("Coherence snapshot:")
        print("  nodes:            42")
        print("  coherence index:  0.875")
        print("  high/mid/low:     40/1/1")
        raise SystemExit(0)
    if argv and argv[0] == "scan":
        print("Registered 3 files. Nodes: 42")
        print("Coherence index: 0.875")
        raise SystemExit(0)
    if argv and argv[0] == "context":
        print('{{"source_documents": 2, "baseline_tokens": 1000, "physis_tokens": 200, "compression_ratio": 0.8, "clusters": ["src"], "physis_context": "packed"}}')
        raise SystemExit(0)
    if argv and argv[0] == "direction":
        print('{{"direction": "shorter", "before_tokens": 12, "after_tokens": 8, "moved": true, "retention": 0.7}}')
        raise SystemExit(0)
    if argv and argv[0] == "node-search":
        print('Fixed-token search: 2 results (120 tokens, budget 300)')
        print('  1. 0.9  src/auth.py: def rotate_token()')
        raise SystemExit(0)
    if argv and argv[0] == "watch":
        print("appended 3 observation(s), seq 1..=3  (3 fs)")
        raise SystemExit(0)
    if argv and argv[0] == "observed":
        print('[{{"seq": 1, "source": "fs", "subject": "a.py"}}, {{"seq": 2, "source": "git", "subject": "c1"}}]')
        raise SystemExit(0)
    if argv and argv[0] == "grid":
        print('{{"domains": ["DEFEND", "HEAL"], "modes": ["TEACH", "WORK"], "shape": [2, 2], "total_cells": 4, "populated_cells": 3, "entries": 9, "expanded": true, "embedder": {{"kind": "onnx-explicit", "dimension": 384}}}}')
        raise SystemExit(0)
    if argv and argv[0] == "chain":
        print('{{"gaps": ["gap-a"], "control": {{"permuted": true}}, "drift": []}}')
        raise SystemExit(0)
    if argv and argv[0] == "classify":
        print("  HEAL        × MAINTAIN   0.940 ")
        print("  CONSTRUCT   × CREATE     0.921 ")
        print("Grid: 8×20 = 160 cells (160 populated, 896 entries)")
        print("Margin over runner-up: 0.019  effectively a tie")
        print("Embedder: onnx-explicit (384d) — semantic")
        if "--no-expand" not in argv:
            print("Ontology expanded: Sample Learned Thing (HEAL×MAINTAIN) — learned entry saved")
        raise SystemExit(0)
    print("ran:" + " ".join(argv))
    raise SystemExit(0)
    """)
    path.write_text(script)
    path.chmod(path.stat().st_mode | stat.S_IEXEC)
    return str(path)


@pytest.fixture(autouse=True)
def _reset_pool():
    reset_pool()
    yield
    reset_pool()


def test_engine_status_reports_bundle(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_engine_status"]({"engine_bin": binary})
    assert result["ok"] is True
    assert binary == result["engines"][0]["binary"]
    assert "scan" in result["engines"][0]["capabilities"]
    assert result["graph"]["nodes"] == 42


def test_observe_scans_and_parses(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_observe"]({"engine_bin": binary, "path": str(tmp_path)})
    assert result["ok"] is True
    assert result["files"] == 3
    assert result["nodes"] == 42
    assert result["coherence_index"] == 0.875


def test_observe_rejects_home(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_observe"]({"engine_bin": binary, "path": "~"})
    assert result["ok"] is False
    assert "home directory" in result["error"]


def test_context_compiles_packet(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_context"](
        {"engine_bin": binary, "path": str(tmp_path), "task": "where is auth", "budget": 250, "mode": "docs"}
    )
    assert result["ok"] is True
    assert result["argv"][1] == "context"
    assert result["physis_tokens"] == 200
    assert result["physis_context"] == "packed"


def test_context_code_mode_uses_graph(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_context"](
        {"engine_bin": binary, "path": str(tmp_path), "task": "where is auth", "budget": 250}
    )
    assert result["ok"] is True
    assert result["argv"][1] == "node-search"
    assert result["results"] == 2
    assert result["mode"] == "code"


def test_context_requires_task(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_context"]({"engine_bin": binary, "path": str(tmp_path)})
    assert result["ok"] is False
    assert "task is required" in result["error"]


def test_query_search_args(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_query"](
        {"engine_bin": binary, "path": str(tmp_path), "question": "token rotation", "limit": 3}
    )
    assert result["ok"] is True
    assert "ran:search token rotation --max 3" in result["stdout"]


def test_why_maps_object_to_node_search(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_why"](
        {"engine_bin": binary, "path": str(tmp_path), "object": "AuthService"}
    )
    assert result["ok"] is True
    assert result["argv"][1] == "node-search"
    assert "AuthService" in result["argv"]
    assert "Evidence retrieval" in result["note"]


def test_classify_parses_cells(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_classify"]({"engine_bin": binary, "text": "def add(): pass"})
    assert result["ok"] is True
    assert result["top_cell"]["cell"] == "HEAL×MAINTAIN"
    assert result["cells"][0]["score"] == 0.94
    assert result["tie"] is True


def test_diff_uses_native_direction(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    before = tmp_path / "before.txt"
    after = tmp_path / "after.txt"
    before.write_text("one two three")
    after.write_text("one")
    result = HANDLERS["physis_diff"](
        {"engine_bin": binary, "before": str(before), "after": str(after), "want": "shorter"}
    )
    assert result["ok"] is True
    assert result["argv"][1] == "direction"
    assert result["moved"] is True
    assert result["direction"] == "shorter"


def test_grid_reports_axes_and_shape(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_grid"]({"engine_bin": binary})
    assert result["ok"] is True
    assert result["shape"] == [2, 2]
    assert result["expanded"] is True
    assert result["embedder"]["kind"] == "onnx-explicit"


def test_classify_reports_expansion(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_classify"]({"engine_bin": binary, "text": "ambiguous thing"})
    assert result["ok"] is True
    assert result["created_entry"] == "Sample Learned Thing"
    assert result["created_cell"] == "HEAL×MAINTAIN"
    assert result["embedder_kind"] == "onnx-explicit"


def test_classify_no_expand_flag(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_classify"](
        {"engine_bin": binary, "text": "ambiguous thing", "auto_expand": False}
    )
    assert result["ok"] is True
    assert "--no-expand" in result["argv"]
    assert "created_entry" not in result


def test_ingest_parses_appended(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_ingest"]({"engine_bin": binary, "path": str(tmp_path), "source": "fs"})
    assert result["ok"] is True
    assert result["appended"] == 3
    assert result["argv"][1] == "watch"


def test_register_tools_uses_plugin_config(tmp_path, monkeypatch):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    settings = {
        "engine_bin": binary,
        "timeout": 7,
        "budget": 123,
        "auto_expand": False,
        "auto_observe": True,
        "model_dir": str(tmp_path / "model"),
    }
    captured = {}

    def fake_run(pool, argv, merged):
        captured.update(merged)
        captured["argv"] = list(argv)
        return {"ok": True, "argv": list(argv), "stdout": ""}

    monkeypatch.setattr(tools, "_run", fake_run)

    class Ctx:
        handlers: dict = {}

        def get_config(self, key, default=None):
            return settings.get(key, default)

        def register_tool(self, name, toolset, schema, handler, description=None):
            self.handlers[name] = handler

    ctx = Ctx()
    ctx.handlers = {}
    names = tools.register_tools(ctx)
    assert "physis_classify" in names
    assert "physis_context" in names

    ctx.handlers["physis_classify"]({"text": "hello"})
    assert captured["engine_bin"] == binary
    assert captured["timeout"] == 7
    assert captured["auto_expand"] is False
    assert "--no-expand" in captured["argv"]

    captured.clear()
    ctx.handlers["physis_context"]({"query": "hello", "path": str(tmp_path)})
    assert captured["budget"] == 123
    assert captured["timeout"] == 7


def test_observed_reads_log(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_observed"]({"engine_bin": binary, "limit": 2})
    assert result["ok"] is True
    assert result["count"] == 2
    assert result["observations"][0]["source"] == "fs"


def test_chain_passes_through_json(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_chain"](
        {"engine_bin": binary, "path": str(tmp_path), "task": "gaps", "budget": 300}
    )
    assert result["ok"] is True
    assert result["argv"][1] == "chain"
    assert result["gaps"] == ["gap-a"]


def test_missing_engine_is_actionable(tmp_path):
    result = HANDLERS["physis_engine_status"]({"engine_bin": str(tmp_path / "nope")})
    assert result["ok"] is False
    assert "not found" in result["error"]
    assert result["hint"] == "run physis_engine_status"


def test_replant_run_requires_existing_registry(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["replant_run"](
        {"engine_bin": binary, "path": str(tmp_path), "input": "x", "tools": str(tmp_path / "missing.json")}
    )
    assert result["ok"] is False
    assert "tools registry not found" in result["error"]


def test_unsupported_command_is_reported(tmp_path):
    binary = make_binary(tmp_path, {"search"})
    result = HANDLERS["physis_observe"]({"engine_bin": binary, "path": str(tmp_path)})
    assert result["ok"] is False
    assert "no available engine advertises 'scan'" in result["error"]


def test_command_passes_stdin(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["physis_command"](
        {"engine_bin": binary, "command": "search", "args": ["x"], "stdin": "{}"}
    )
    assert result["ok"] is True


# --- hook contract ---------------------------------------------------------- #


def make_recording_engine(tmp_path: Path) -> tuple[str, Path]:
    """Engine that appends its argv to a log, so a hook can be observed."""
    log = tmp_path / "calls.log"
    path = tmp_path / "recording-engine"
    script = textwrap.dedent(f"""\
    #!/usr/bin/env python3
    import sys
    with open({str(log)!r}, "a") as handle:
        handle.write(" ".join(sys.argv[1:]) + "\\n")
    if sys.argv[1:] == ["--help"]:
        print("Commands:\\n  scan  Scan a directory")
    raise SystemExit(0)
    """)
    path.write_text(script)
    path.chmod(path.stat().st_mode | stat.S_IEXEC)
    return str(path), log


def test_post_tool_call_reads_args_kwarg(tmp_path):
    """Hermes dispatches ``args``; reading only ``tool_args`` silently disabled the hook."""
    binary, log = make_recording_engine(tmp_path)
    target = tmp_path / "pkg" / "mod.py"
    target.parent.mkdir()
    target.write_text("x = 1\n")

    hook = tools.make_post_tool_call({"auto_observe": True, "engine_bin": binary, "timeout": 5})
    assert hook(tool_name="write_file", args={"path": str(target)}, result="{}") is None

    calls = [line for line in log.read_text().splitlines() if line.startswith("scan ")]
    assert calls, log.read_text()
    assert str(target.parent) in calls[0]


def test_post_tool_call_rereads_live_config(tmp_path):
    binary, log = make_recording_engine(tmp_path)
    target = tmp_path / "mod.py"
    target.write_text("x = 1\n")

    class Ctx:
        settings = {"auto_observe": False, "engine_bin": binary}

        def get_config(self, key, default=None):
            return self.settings.get(key, default)

    ctx = Ctx()
    hook = tools.make_post_tool_call(ctx)
    hook(tool_name="write_file", args={"path": str(target)})
    logged = log.read_text() if log.exists() else ""
    assert "scan " not in logged

    ctx.settings["auto_observe"] = True  # enabled after load
    hook(tool_name="write_file", args={"path": str(target)})
    assert "scan " in log.read_text()


# --- pre_verify ------------------------------------------------------------- #


def test_pre_verify_nudges_on_failure_only(tmp_path):
    target = tmp_path / "mod.py"
    target.write_text("x = 1\n")
    config = {"verify_cmd": "exit 3", "timeout": 10}

    result = tools.make_pre_verify(config)(changed_paths=[str(target)], attempt=0)
    assert result["action"] == "continue"
    assert "exit 3" in result["message"]

    assert tools.make_pre_verify({"verify_cmd": "true"})(changed_paths=[str(target)]) is None
    assert tools.make_pre_verify(config)(changed_paths=[str(target)], attempt=1) is None
    assert tools.make_pre_verify({"verify_cmd": ""})(changed_paths=[str(target)]) is None


# --- artifact freshness ------------------------------------------------------ #


def test_artifacts_are_fingerprinted_and_detect_change(tmp_path):
    target = tmp_path / "input.txt"
    target.write_text("one")
    args = {"input": "deploy", "path": str(tmp_path), "store": str(tmp_path / ".replant")}

    assert tools._changed_artifacts(args) == []

    tools._write_artifacts(args, [str(target)])
    assert tools._changed_artifacts(args) == []

    target.write_text("two")
    assert tools._changed_artifacts(args) == [str(target)]

    # Re-recording without artifacts drops the old set instead of blocking forever.
    tools._write_artifacts(args, [])
    assert tools._changed_artifacts(args) == []


def test_replant_run_blocks_when_inputs_changed(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    registry = tmp_path / "tools.json"
    registry.write_text("{}")
    target = tmp_path / "input.txt"
    target.write_text("one")
    args = {
        "engine_bin": binary,
        "path": str(tmp_path),
        "input": "deploy",
        "tools": str(registry),
        "store": str(tmp_path / ".replant"),
    }
    tools._write_artifacts(args, [str(target)])
    target.write_text("two")

    result = HANDLERS["replant_run"](args)
    assert result["ok"] is False
    assert result["blocked"] is True
    assert result["provider_calls"] == 0
    assert result["changed"] == [str(target)]


# --- /physis command -------------------------------------------------------- #


def test_physis_command_usage_and_unknown_verb():
    handler = tools.make_physis_command({})
    assert "Usage: /physis" in handler("nope")
    assert handler("replay") == "Usage: /physis replay <prompt>"
