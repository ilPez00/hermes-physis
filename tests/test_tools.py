import json
import os
import re
import stat
import sys
import textwrap
import time
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
    ctx.handlers["physis_context"]({"task": "hello", "path": str(tmp_path)})
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


def test_replant_run_requires_an_operator_registry(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    result = HANDLERS["replant_run"](
        {"engine_bin": binary, "path": str(tmp_path), "input": "x"}
    )
    assert result["ok"] is False
    assert "replant_tools" in result["error"]
    assert "never taken from a repository path" in result["hint"]


def test_unsupported_command_is_reported(tmp_path):
    binary = make_binary(tmp_path, {"search"})
    result = HANDLERS["physis_observe"]({"engine_bin": binary, "path": str(tmp_path)})
    assert result["ok"] is False
    assert "no available engine advertises 'scan'" in result["error"]


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
    args = {"input": "deploy", "path": str(tmp_path)}

    with pytest.raises(ValueError, match="freshness evidence"):
        tools._changed_artifacts(args)

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
        "replant_tools": str(registry),  # operator config, not a model argument
    }
    tools._write_artifacts(args, [str(target)])
    target.write_text("two")

    result = HANDLERS["replant_run"](args)
    assert result["ok"] is False
    assert result["blocked"] is True
    assert result["provider_calls"] == 0
    assert result["changed"] == [str(target)]


# --- learned workflow candidates -------------------------------------------- #


def test_verified_session_becomes_a_candidate(tmp_path):
    tools.reset_learned()
    target = tmp_path / "out.txt"
    target.write_text("v1")
    config = {"auto_learn": True, "verify_cmd": "true", "timeout": 10}

    tools.make_pre_llm_call(config)(session_id="s1", user_message="deploy the api")
    post = tools.make_post_tool_call(config)
    post(tool_name="terminal", args={"command": "echo build", "workdir": str(tmp_path)}, session_id="s1")
    post(tool_name="write_file", args={"path": str(target)}, session_id="s1")
    # A failed check must not learn anything.
    pre = tools.make_pre_verify({**config, "verify_cmd": "exit 1"})
    assert pre(changed_paths=[str(target)], session_id="s1")["action"] == "continue"
    assert tools.physis_candidates({"path": str(tmp_path)})["count"] == 0

    assert tools.make_pre_verify(config)(changed_paths=[str(target)], session_id="s1") is None
    listed = tools.physis_candidates({"path": str(tmp_path)})
    assert listed["count"] == 1
    candidate = listed["candidates"][0]
    assert candidate["prompt"] == "deploy the api"
    assert candidate["steps"] == [{"command": "echo build", "workdir": str(tmp_path)}]
    assert candidate["files"] == [str(target.resolve())]
    assert candidate["check"] == "true"


def test_candidate_without_check_or_intent_is_not_saved(tmp_path):
    tools.reset_learned()
    target = tmp_path / "out.txt"
    target.write_text("v1")
    pre = tools.make_pre_verify({"auto_learn": True, "verify_cmd": "true", "timeout": 10})
    post = tools.make_post_tool_call({"auto_learn": True})

    post(tool_name="terminal", args={"command": "echo hi"}, session_id="no-intent")
    pre(changed_paths=[str(target)], session_id="no-intent")
    assert tools.physis_candidates({"path": str(tmp_path)})["count"] == 0


def test_approve_records_and_replays_then_blocks_on_change(tmp_path, monkeypatch):
    tools.reset_learned()
    artifact = tmp_path / "input.txt"
    artifact.write_text("v1")
    config = {"auto_learn": True, "verify_cmd": "test -f input.txt", "timeout": 30}

    tools.make_pre_llm_call(config)(session_id="s2", user_message="append a step")
    tools.make_post_tool_call(config)(
        tool_name="terminal",
        args={"command": "printf s >> steps.log", "workdir": str(tmp_path)},
        session_id="s2",
    )
    tools.make_post_tool_call(config)(
        tool_name="write_file", args={"path": str(artifact)}, session_id="s2"
    )
    assert tools.make_pre_verify(config)(changed_paths=[str(artifact)], session_id="s2") is None
    candidate = tools.physis_candidates({"path": str(tmp_path)})["candidates"][0]

    # Approving is operator-only: the model's replant_record refuses a candidate.
    refused = tools.replant_record({"candidate": candidate["id"]})
    assert refused["ok"] is False
    assert "approve" in refused["error"]

    monkeypatch.chdir(tmp_path)
    command = tools.make_physis_command(config)
    approved = json.loads(command("approve " + candidate["id"]))
    assert approved["ok"] is True, approved
    assert approved["status"] == "succeeded"
    assert approved["provider_calls"] == 0
    assert approved["candidate_id"] == candidate["id"]

    args = {"input": "append a step"}  # registry resolves to the learned one
    assert HANDLERS["replant_run"](args)["operation"] == "replay"

    artifact.write_text("v2")  # the ground moves
    blocked = HANDLERS["replant_run"](args)
    assert blocked["blocked"] is True
    assert blocked["changed"] == [str(artifact)]


def test_approve_unknown_candidate_is_actionable(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    result = json.loads(tools.make_physis_command({})("approve deadbeef"))
    assert result["ok"] is False
    assert "candidate not found" in result["error"]
    assert result["hint"] == "run physis_candidates"


def test_command_replay_finds_the_learned_registry(tmp_path, monkeypatch):
    """An approved candidate writes learned/tools.json; /physis replay must find it."""
    artifact = tmp_path / "input.txt"
    artifact.write_text("v1")
    tools.reset_learned()
    config = {"auto_learn": True, "verify_cmd": "true", "timeout": 30}
    tools.make_pre_llm_call(config)(session_id="s3", user_message="greet")
    tools.make_post_tool_call(config)(
        tool_name="terminal", args={"command": "echo hi", "workdir": str(tmp_path)}, session_id="s3"
    )
    tools.make_post_tool_call(config)(
        tool_name="write_file", args={"path": str(artifact)}, session_id="s3"
    )
    tools.make_pre_verify(config)(changed_paths=[str(artifact)], session_id="s3")
    candidate = tools.physis_candidates({"path": str(tmp_path)})["candidates"][0]

    monkeypatch.chdir(tmp_path)
    handler = tools.make_physis_command({"timeout": 30})
    assert json.loads(handler("approve " + candidate["id"]))["ok"] is True
    replayed = json.loads(handler("replay greet"))
    assert replayed["operation"] == "replay"
    assert replayed["provider_calls"] == 0


# --- /physis command -------------------------------------------------------- #


def test_physis_command_usage_and_unknown_verb():
    handler = tools.make_physis_command({})
    assert "Usage: /physis" in handler("nope")
    assert handler("replay") == "Usage: /physis replay <prompt>"
    assert handler("approve") == "Usage: /physis approve <candidate-id>"


# --- canonical repository root ---------------------------------------------- #


def _make_repo(tmp_path: Path, name: str = "repo") -> Path:
    repo = tmp_path / name
    (repo / ".git").mkdir(parents=True)
    return repo


def test_repo_root_is_the_nearest_vcs_marker(tmp_path):
    repo = _make_repo(tmp_path)
    nested = repo / "src" / "auth"
    nested.mkdir(parents=True)
    target = nested / "mod.py"
    target.write_text("x = 1\n")
    assert tools._repo_root(str(target)) == str(repo)
    assert tools._repo_root(str(nested)) == str(repo)
    assert tools._repo_root(str(repo)) == str(repo)


def test_repo_root_falls_back_to_the_path(tmp_path):
    plain = tmp_path / "plain"
    plain.mkdir()
    assert tools._repo_root(str(plain)) == str(plain)


def test_index_env_is_one_store_per_repository(tmp_path, monkeypatch):
    monkeypatch.delenv("PHYSIS_CORE_DIR", raising=False)
    repo = _make_repo(tmp_path)
    nested = repo / "src"
    nested.mkdir()
    root_env = tools._index_env({"path": str(repo)})
    sub_env = tools._index_env({"path": str(nested)})
    assert root_env == sub_env
    store = root_env["PHYSIS_CORE_DIR"]
    assert store.startswith(str(tools._profile_dir()))
    assert Path(store).parent.name == "repos"
    assert re.fullmatch(r"[0-9a-f]{12}", Path(store).name)


def test_index_env_separates_repositories(tmp_path, monkeypatch):
    monkeypatch.delenv("PHYSIS_CORE_DIR", raising=False)
    first = _make_repo(tmp_path, "first")
    second = _make_repo(tmp_path, "second")
    assert tools._index_env({"path": str(first)}) != tools._index_env({"path": str(second)})


def test_observe_reports_the_repository_root(tmp_path):
    binary = make_binary(tmp_path, CORE_COMMANDS)
    repo = _make_repo(tmp_path)
    (repo / "src").mkdir()
    result = HANDLERS["physis_observe"]({"engine_bin": binary, "path": str(repo / "src")})
    assert result["ok"] is True
    # No .git under src, so the nearest marker is still the repository root.
    assert result["repo_root"] == str(repo)
    assert result["store"]


def test_post_tool_call_indexes_into_the_repository_store(tmp_path):
    repo = _make_repo(tmp_path)
    target = repo / "src" / "mod.py"
    target.parent.mkdir()
    target.write_text("x = 1\n")
    log = tmp_path / "store.log"
    binary = tmp_path / "store-engine"
    binary.write_text(
        textwrap.dedent(f"""\
        #!/usr/bin/env python3
        import os, sys
        if sys.argv[1:] == ["--help"]:
            print("Commands:\\n  scan  Scan")
            raise SystemExit(0)
        with open({str(log)!r}, "a") as handle:
            handle.write(os.environ.get("PHYSIS_CORE_DIR", "") + "\\n")
        raise SystemExit(0)
        """)
    )
    binary.chmod(binary.stat().st_mode | stat.S_IEXEC)

    hook = tools.make_post_tool_call({"auto_observe": True, "engine_bin": str(binary), "timeout": 5})
    assert hook(tool_name="write_file", args={"path": str(target)}, session_id="s1") is None
    assert log.read_text().strip() == tools._index_dir({"path": str(repo)})


# --- pre_verify directory and timeout --------------------------------------- #


def test_pre_verify_runs_from_the_repository_root(tmp_path):
    repo = _make_repo(tmp_path)
    target = repo / "src" / "mod.py"
    target.parent.mkdir()
    target.write_text("x = 1\n")
    probe = f'{sys.executable} -c "import os; print(os.getcwd()); raise SystemExit(3)"'
    result = tools.make_pre_verify({"verify_cmd": probe, "timeout": 15})(
        changed_paths=[str(target)], attempt=0
    )
    assert result["action"] == "continue"
    assert str(repo) in result["message"]


def test_pre_verify_timeout_is_reported_not_ignored(tmp_path):
    target = tmp_path / "mod.py"
    target.write_text("x = 1\n")
    hook = tools.make_pre_verify({"verify_cmd": "sleep 5", "verify_timeout": 1, "timeout": 15})
    result = hook(changed_paths=[str(target)], attempt=0)
    assert result["action"] == "continue"
    assert "timed out" in result["message"]


def test_pre_verify_timeout_is_separate_from_the_engine_timeout(tmp_path):
    from tools import DEFAULTS

    assert 0 < DEFAULTS["verify_timeout"] <= 30  # the host bounds this hook at 30s
    target = tmp_path / "mod.py"
    target.write_text("x = 1\n")
    # ``timeout`` is large but ``verify_timeout`` is small: the small one wins.
    hook = tools.make_pre_verify({"verify_cmd": "sleep 5", "verify_timeout": 1, "timeout": 300})
    assert "timed out after 1s" in hook(changed_paths=[str(target)])["message"]


# --- pre-reasoning context --------------------------------------------------- #


def _node_search_engine(tmp_path: Path, body: str) -> str:
    binary = tmp_path / "ns-engine"
    binary.write_text(
        "#!/usr/bin/env python3\n"
        "import sys\n"
        "if sys.argv[1:] == ['--help']:\n"
        "    print('Commands:\\n  node-search  Search')\n"
        "    raise SystemExit(0)\n" + body
    )
    binary.chmod(binary.stat().st_mode | stat.S_IEXEC)
    return str(binary)


def _fresh_store(tmp_path: Path, repo: Path, *, stale: bool = False) -> Path:
    store = Path(tools._index_dir({"path": str(repo)}))
    store.mkdir(parents=True, exist_ok=True)
    marker = store / "graph.json"
    marker.write_text("{}")
    when = time.time() - 3600 if stale else time.time() + 3600
    os.utime(marker, (when, when))
    return store


def _bind_session(repo: Path, session: str) -> None:
    """Stand in for the post_tool_call hook recording the session's repository."""
    tools._remember_root(session, str(repo))


def test_pre_context_injects_from_a_fresh_index(tmp_path, monkeypatch):
    monkeypatch.delenv("PHYSIS_CORE_DIR", raising=False)
    repo = _make_repo(tmp_path)
    (repo / "auth.py").write_text("def rotate_token():\n    pass\n")
    _fresh_store(tmp_path, repo)
    _bind_session(repo, "ctx1")

    binary = _node_search_engine(
        tmp_path,
        f'print("Fixed-token search: 1 results (30 tokens, budget 300)")\n'
        f'print("  1. 0.91  {repo}/auth.py: def rotate_token() (30 tokens)")\n',
    )
    hook = tools.make_pre_llm_call({"pre_context": True, "engine_bin": binary, "timeout": 10})
    result = hook(session_id="ctx1", user_message="rotate the auth token")
    assert result and "context" in result
    assert "auth.py" in result["context"]
    assert "0.91" in result["context"]
    assert "Uncertainty" in result["context"]

    # The identical packet is not injected twice in one session.
    assert hook(session_id="ctx1", user_message="rotate the auth token") is None


def test_pre_context_falls_back_to_the_process_directory(tmp_path, monkeypatch):
    monkeypatch.delenv("PHYSIS_CORE_DIR", raising=False)
    repo = _make_repo(tmp_path)
    (repo / "auth.py").write_text("def rotate_token():\n    pass\n")
    _fresh_store(tmp_path, repo)
    monkeypatch.chdir(repo)
    binary = _node_search_engine(
        tmp_path,
        'print("Fixed-token search: 1 results (30 tokens, budget 300)")\n'
        'print("  1. 0.91  auth.py: def rotate_token() (30 tokens)")\n',
    )
    hook = tools.make_pre_llm_call({"pre_context": True, "engine_bin": binary, "timeout": 10})
    result = hook(session_id="ctx-cwd", user_message="rotate the auth token")
    assert result and "auth.py" in result["context"]


def test_pre_context_skips_a_stale_index(tmp_path, monkeypatch):
    monkeypatch.delenv("PHYSIS_CORE_DIR", raising=False)
    repo = _make_repo(tmp_path)
    (repo / "auth.py").write_text("def rotate_token():\n    pass\n")
    _fresh_store(tmp_path, repo, stale=True)
    _bind_session(repo, "ctx2")
    binary = _node_search_engine(
        tmp_path,
        f'print("Fixed-token search: 1 results (30 tokens, budget 300)")\n'
        f'print("  1. 0.91  {repo}/auth.py: def rotate_token() (30 tokens)")\n',
    )
    hook = tools.make_pre_llm_call({"pre_context": True, "engine_bin": binary, "timeout": 10})
    assert hook(session_id="ctx2", user_message="rotate the auth token") is None


def test_pre_context_skips_weak_evidence(tmp_path, monkeypatch):
    monkeypatch.delenv("PHYSIS_CORE_DIR", raising=False)
    repo = _make_repo(tmp_path)
    (repo / "auth.py").write_text("def rotate_token():\n    pass\n")
    _fresh_store(tmp_path, repo)
    _bind_session(repo, "ctx3")
    binary = _node_search_engine(
        tmp_path,
        f'print("Fixed-token search: 1 results (30 tokens, budget 300)")\n'
        f'print("  1. 0.21  {repo}/auth.py: def rotate_token() (30 tokens)")\n',
    )
    hook = tools.make_pre_llm_call({"pre_context": True, "engine_bin": binary, "timeout": 10})
    assert hook(session_id="ctx3", user_message="rotate the auth token") is None


def test_pre_context_off_by_default(tmp_path):
    hook = tools.make_pre_llm_call({})
    assert hook(session_id="ctx4", user_message="rotate the auth token") is None


def test_pre_context_reads_the_index_and_never_ingests(tmp_path, monkeypatch):
    monkeypatch.delenv("PHYSIS_CORE_DIR", raising=False)
    repo = _make_repo(tmp_path)
    (repo / "auth.py").write_text("def rotate_token():\n    pass\n")
    _fresh_store(tmp_path, repo)
    _bind_session(repo, "ctx5")
    log = tmp_path / "calls.log"
    binary = tmp_path / "logging-engine"
    binary.write_text(
        textwrap.dedent(f"""\
        #!/usr/bin/env python3
        import sys
        if sys.argv[1:] == ["--help"]:
            print("Commands:\\n  node-search  Search\\n  watch  Watch")
            raise SystemExit(0)
        with open({str(log)!r}, "a") as handle:
            handle.write(" ".join(sys.argv[1:]) + "\\n")
        print("Fixed-token search: 1 results (30 tokens, budget 300)")
        print("  1. 0.91  auth.py: def rotate_token() (30 tokens)")
        raise SystemExit(0)
        """)
    )
    binary.chmod(binary.stat().st_mode | stat.S_IEXEC)
    hook = tools.make_pre_llm_call({"pre_context": True, "engine_bin": binary, "timeout": 10})
    assert hook(session_id="ctx5", user_message="rotate the auth token")
    calls = log.read_text().splitlines()
    assert calls == ["node-search rotate the auth token --budget 300 --max 5"]


def test_pre_context_fails_open_without_a_store(tmp_path, monkeypatch):
    monkeypatch.delenv("PHYSIS_CORE_DIR", raising=False)
    repo = _make_repo(tmp_path)
    binary = _node_search_engine(tmp_path, 'print("Fixed-token search: 0 results")\n')
    hook = tools.make_pre_llm_call({"pre_context": True, "engine_bin": binary, "timeout": 10})
    assert hook(session_id="ctx6", user_message="rotate the auth token") is None


def test_session_root_is_remembered_from_edits(tmp_path):
    repo = _make_repo(tmp_path)
    target = repo / "src" / "mod.py"
    target.parent.mkdir()
    target.write_text("x = 1\n")
    binary = make_binary(tmp_path, CORE_COMMANDS)
    hook = tools.make_post_tool_call({"auto_observe": True, "engine_bin": binary, "timeout": 5})
    hook(tool_name="write_file", args={"path": str(target)}, session_id="sess")
    assert tools._session_root("sess") == str(repo)


# --- read-only recipe lookup ------------------------------------------------- #


def test_recipe_lookup_requires_an_input(tmp_path):
    result = tools.physis_recipe_lookup({"path": str(tmp_path)})
    assert result["ok"] is False
    assert "input is required" in result["error"]


def test_recipe_lookup_without_a_registry_is_not_found(tmp_path):
    result = tools.physis_recipe_lookup({"input": "deploy the api", "path": str(tmp_path)})
    assert result["ok"] is True
    assert result["found"] is False
    assert result["replayable"] is False
    assert "registry" in result["reason"]


def test_recipe_lookup_rejects_a_model_supplied_registry(tmp_path):
    result = tools.physis_recipe_lookup({"input": "x", "tools": "/tmp/evil.json", "path": str(tmp_path)})
    assert result["ok"] is False
    assert "operator" in result["error"] or "approve" in result["error"]
