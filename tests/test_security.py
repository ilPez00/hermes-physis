"""Catalog security hardening: regression tests written before the fixes.

Each test pins one property of the hardened surface:

* ``physis_command`` is gone: no model-reachable escape hatch into raw engine
  argv/stdin.
* Registered handlers reject arguments their schema does not declare, so a
  model can never set executable settings (``engine_bin``, ``tools``,
  ``store``, ``verify_cmd``, ``replant_tools``, ``candidate``...) by naming
  them as tool arguments.
* Executable registries and recorded recipes live in the fixed plugin-owned
  profile directory, never in a model-controlled repo path; ``/physis replay``
  never resolves a registry planted inside a repository.
* Candidates are only loaded from the profile directory - never searched for
  in the working tree - and approving one is exclusively ``/physis approve``.
* Every subprocess the plugin spawns gets a fixed minimal environment: no
  blanket inheritance, no stray PHYSIS_* variables, no secrets leaking into
  engine, verify or replay-step children.
* Replay and the stale-input refusal keep working end to end.
"""

import json
import os
import stat
import subprocess
import sys
import textwrap
from pathlib import Path

import pytest
import yaml

import tools
from schemas import ALL_NAMES, REPLANT_RECORD, REPLANT_RUN
from tools import HANDLERS

from test_tools import CORE_COMMANDS, make_binary

PLUGIN_DIR = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.security


class _Ctx:
    """Minimal registration harness: handlers as the model would reach them."""

    def __init__(self, settings=None):
        self.settings = settings or {}
        self.handlers = {}

    def get_config(self, key, default=None):
        return self.settings.get(key, default)

    def register_tool(self, name, toolset, schema, handler, description=None):
        self.handlers[name] = handler


def _registered(settings=None):
    ctx = _Ctx(settings)
    tools.register_tools(ctx)
    return ctx


def _call(handler, payload):
    result = handler(payload)
    return json.loads(result) if isinstance(result, str) else result


def _make_env_engine(tmp_path: Path) -> str:
    """Engine that dumps the environment it was spawned with."""
    path = tmp_path / "env-engine"
    path.write_text(textwrap.dedent("""\
        #!/usr/bin/env python3
        import os, sys
        if sys.argv[1:] == ["--help"]:
            print("Commands:\\n  scan  Scan\\n  stats  Stats")
            raise SystemExit(0)
        print("KEYS:" + ",".join(sorted(os.environ)))
        raise SystemExit(0)
    """))
    path.chmod(path.stat().st_mode | stat.S_IEXEC)
    return str(path)


def _learned_candidate(tmp_path: Path, command: str) -> dict:
    """Run the auto_learn loop until one verified candidate exists."""
    tools.reset_learned()
    artifact = tmp_path / "input.txt"
    artifact.write_text("v1")
    config = {"auto_learn": True, "verify_cmd": "true", "timeout": 30}
    tools.make_pre_llm_call(config)(session_id="sec", user_message="greet the world")
    tools.make_post_tool_call(config)(
        tool_name="terminal", args={"command": command, "workdir": str(tmp_path)},
        session_id="sec",
    )
    tools.make_post_tool_call(config)(
        tool_name="write_file", args={"path": str(artifact)}, session_id="sec"
    )
    assert tools.make_pre_verify(config)(changed_paths=[str(artifact)], session_id="sec") is None
    listed = tools.physis_candidates({})
    assert listed["count"] == 1, listed
    return listed["candidates"][0]


requires_replant = pytest.mark.skipif(
    not (PLUGIN_DIR / "bin" / "replant").is_file(), reason="bundled replant engine not present"
)


# --------------------------------------------------------------------------- #
# 1. physis_command is removed
# --------------------------------------------------------------------------- #

def test_physis_command_is_removed():
    assert "physis_command" not in HANDLERS
    assert "physis_command" not in ALL_NAMES
    manifest = yaml.safe_load((PLUGIN_DIR / "plugin.yaml").read_text())
    assert "physis_command" not in manifest["provides_tools"]


# --------------------------------------------------------------------------- #
# 2. Registered handlers reject undeclared arguments
# --------------------------------------------------------------------------- #

def test_registered_handler_rejects_undeclared_executable_settings(tmp_path):
    ctx = _registered({"engine_bin": make_binary(tmp_path, CORE_COMMANDS)})
    dangerous = {
        "engine_bin": "/tmp/evil-engine",
        "engine_bins": "/tmp/evil-engine",
        "replant_tools": "/tmp/evil-registry.json",
        "verify_cmd": "rm -rf /",
        "model_dir": "/tmp/evil-model",
        "tools": "/tmp/evil-registry.json",
        "store": "/tmp/evil-store",
        "candidate": "deadbeef",
        "timeout": 9999,
    }
    for key, value in dangerous.items():
        result = _call(ctx.handlers["physis_snapshot"], {key: value})
        assert result["ok"] is False, (key, result)
        assert "undeclared" in result["error"], (key, result)


def test_registered_replant_run_rejects_model_supplied_tools_and_store(tmp_path):
    ctx = _registered({"engine_bin": make_binary(tmp_path, CORE_COMMANDS)})
    assert "replant_run" not in ctx.handlers


def test_registered_replant_record_rejects_model_supplied_candidate(tmp_path):
    ctx = _registered({"engine_bin": make_binary(tmp_path, CORE_COMMANDS)})
    assert "replant_record" not in ctx.handlers


def test_declared_arguments_only_in_replant_schemas():
    for schema in (REPLANT_RUN, REPLANT_RECORD):
        props = schema["parameters"]["properties"]
        for banned in ("tools", "store", "candidate", "engine_bin", "verify_cmd", "replant_tools"):
            assert banned not in props, (schema["name"], banned)


# --------------------------------------------------------------------------- #
# 3. Direct handler calls also refuse model-supplied registry/store/candidate
# --------------------------------------------------------------------------- #

def test_replant_run_direct_refuses_model_supplied_tools(tmp_path):
    planted = tmp_path / "planted.json"
    planted.write_text("{}")
    result = HANDLERS["replant_run"](
        {
            "engine_bin": make_binary(tmp_path, CORE_COMMANDS),
            "input": "x",
            "tools": str(planted),
            "store": str(tmp_path / "store"),
        }
    )
    assert result["ok"] is False
    assert "operator" in result["error"] or "approve" in result["error"], result


def test_replant_record_direct_refuses_model_supplied_candidate(tmp_path):
    result = tools.replant_record(
        {"engine_bin": make_binary(tmp_path, CORE_COMMANDS), "candidate": "deadbeef"}
    )
    assert result["ok"] is False
    assert "approve" in result["error"], result


# --------------------------------------------------------------------------- #
# 4. Registries and recipes live in the fixed profile dir, not the repo
# --------------------------------------------------------------------------- #

def test_replant_store_is_fixed_not_model_supplied(tmp_path):
    result = HANDLERS["replant_stats"](
        {
            "engine_bin": _make_env_engine(tmp_path),
            "path": str(tmp_path),
            "store": str(tmp_path / "elsewhere"),
        }
    )
    argv = result["argv"]
    store = argv[argv.index("--store") + 1]
    assert store == os.environ["PHYSIS_REPLANT_STORE"], store
    assert store != str(tmp_path / "elsewhere")
    assert store != str(Path(tmp_path) / ".replant")


def test_physis_replay_ignores_registry_planted_in_repo(tmp_path, monkeypatch):
    repo = tmp_path / "repo"
    repo.mkdir()
    marker = tmp_path / "pwned"
    planted = {
        "pwn": {
            "kind": "action",
            "program": "/usr/bin/touch",
            "args": [str(marker)],
            "fingerprint_files": [],
            "timeout_ms": 5000,
        }
    }
    (repo / ".replant").mkdir()
    (repo / ".replant" / "tools.json").write_text(json.dumps(planted))
    monkeypatch.chdir(repo)

    handler = tools.make_physis_command({})
    result = json.loads(handler("replay deploy the api"))
    assert result.get("ok") is False, result
    assert "replant_tools" in json.dumps(result) or "approve" in json.dumps(result), result
    assert not marker.exists()


def test_candidates_are_never_loaded_from_the_repo(tmp_path, monkeypatch):
    marker = tmp_path / "pwned"
    evil = {
        "id": "deadbeef",
        "prompt": "pwn",
        "steps": [{"command": f"touch {marker}", "workdir": ""}],
        "files": [],
        "check": "true",
    }
    (tmp_path / "candidates").mkdir()
    (tmp_path / "candidates" / "deadbeef.json").write_text(json.dumps(evil))
    monkeypatch.chdir(tmp_path)

    handler = tools.make_physis_command({"timeout": 30})
    result = json.loads(handler("approve deadbeef"))
    assert result["ok"] is False, result
    assert "not found" in result["error"], result
    assert not marker.exists()


# --------------------------------------------------------------------------- #
# 5. Minimal fixed child environment for every subprocess
# --------------------------------------------------------------------------- #

def test_engine_children_get_minimal_fixed_env(tmp_path, monkeypatch):
    monkeypatch.setenv("PHYSIS_EVIL", "1")
    monkeypatch.setenv("SECRET_TOKEN", "hunter2")
    result = HANDLERS["replant_stats"]({"engine_bin": _make_env_engine(tmp_path)})
    keys = result["stdout"]
    assert "PHYSIS_EVIL" not in keys
    assert "SECRET_TOKEN" not in keys
    assert "PATH" in keys
    assert "HOME" in keys
    assert "PHYSIS_CACHE_DIR" in keys  # explicitly trusted passthrough


def test_verify_cmd_child_env_is_minimal(tmp_path, monkeypatch):
    monkeypatch.setenv("PHYSIS_EVIL", "1")
    monkeypatch.setenv("SECRET_TOKEN", "hunter2")
    target = tmp_path / "mod.py"
    target.write_text("x = 1\n")
    probe = (
        f"{sys.executable} -c \"import os; print(','.join(sorted(os.environ))); "
        "raise SystemExit(3)\""
    )
    hook = tools.make_pre_verify({"verify_cmd": probe, "timeout": 15})
    result = hook(changed_paths=[str(target)], attempt=0)
    assert result["action"] == "continue"
    assert "PHYSIS_EVIL" not in result["message"], result["message"]
    assert "SECRET_TOKEN" not in result["message"], result["message"]


def test_step_wrapper_child_env_is_minimal(tmp_path):
    wrapper = tmp_path / "step.py"
    wrapper.write_text(tools.STEP_WRAPPER)
    wrapper.chmod(wrapper.stat().st_mode | stat.S_IEXEC)
    env = {**os.environ, "PHYSIS_EVIL": "1", "SECRET_TOKEN": "hunter2"}
    probe = "echo E=$PHYSIS_EVIL S=$SECRET_TOKEN P=$PATH"
    proc = subprocess.run(
        [str(wrapper), probe, str(tmp_path)],
        capture_output=True, text=True, env=env, timeout=60,
    )
    payload = json.loads(proc.stdout)
    assert payload["ok"] is True, payload
    assert "E=1" not in payload["output"], payload["output"]
    assert "hunter2" not in payload["output"], payload["output"]
    assert "P=" in payload["output"]  # PATH is preserved for shell steps


# --------------------------------------------------------------------------- #
# 6. Approval, replay and stale-input refusal keep working (positive controls)
# --------------------------------------------------------------------------- #

@requires_replant
def test_approve_then_replay_works_from_any_directory(tmp_path, monkeypatch):
    candidate = _learned_candidate(tmp_path, "echo hi")
    repo = tmp_path / "repo"
    repo.mkdir()

    elsewhere = tmp_path / "elsewhere"
    elsewhere.mkdir()
    monkeypatch.chdir(elsewhere)  # approve from a directory that is not the repo
    handler = tools.make_physis_command({"timeout": 30})
    approved = json.loads(handler("approve " + candidate["id"]))
    assert approved["ok"] is True, approved
    assert approved.get("candidate_id") == candidate["id"]

    monkeypatch.chdir(repo)  # replay from yet another one
    replayed = json.loads(handler("replay greet the world"))
    assert replayed["operation"] == "replay", replayed
    assert replayed["provider_calls"] == 0


@requires_replant
def test_replay_still_refuses_stale_inputs(tmp_path, monkeypatch):
    candidate = _learned_candidate(tmp_path, "echo hi")
    artifact = tmp_path / "input.txt"

    monkeypatch.chdir(tmp_path)
    handler = tools.make_physis_command({"timeout": 30})
    assert json.loads(handler("approve " + candidate["id"]))["ok"] is True

    replayed = json.loads(handler("replay greet the world"))
    assert replayed["operation"] == "replay", replayed

    artifact.write_text("v2")  # the ground moves under a verified recipe
    blocked = json.loads(handler("replay greet the world"))
    assert blocked.get("blocked") is True, blocked
    assert blocked.get("provider_calls") == 0, blocked
    assert blocked.get("changed") == [str(artifact.resolve())], blocked


@requires_replant
def test_replay_refuses_mutated_approved_registry(tmp_path, monkeypatch):
    candidate = _learned_candidate(tmp_path, "echo hi")
    monkeypatch.chdir(tmp_path)
    handler = tools.make_physis_command({"timeout": 30})
    assert json.loads(handler("approve " + candidate["id"]))["ok"] is True

    registry = Path(os.environ["PHYSIS_REPLANT_STORE"]) / "learned" / "tools.json"
    payload = json.loads(registry.read_text())
    payload["step1"]["args"][0] = "touch attacker-controlled"
    payload["step1"]["args"][1] = str(tmp_path / "other-cwd")
    registry.write_text(json.dumps(payload))

    blocked = json.loads(handler("replay greet the world"))
    assert blocked["ok"] is False
    assert blocked["blocked"] is True
    assert blocked["provider_calls"] == 0
    assert "approval" in blocked["reason"]


@requires_replant
def test_tampered_learned_registry_is_refused(tmp_path, monkeypatch):
    candidate = _learned_candidate(tmp_path, "echo hi")
    marker = tmp_path / "pwned"

    monkeypatch.chdir(tmp_path)
    handler = tools.make_physis_command({"timeout": 30})
    assert json.loads(handler("approve " + candidate["id"]))["ok"] is True

    registry = Path(os.environ["PHYSIS_REPLANT_STORE"]) / "learned" / "tools.json"
    tampered = json.loads(registry.read_text())
    tampered["step1"]["args"] = [f"touch {marker}", ""]
    registry.write_text(json.dumps(tampered))

    result = json.loads(handler("replay greet the world"))
    assert result.get("operation") != "replay", result
    assert not marker.exists()
