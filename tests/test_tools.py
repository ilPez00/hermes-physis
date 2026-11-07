import json
import stat
import textwrap
from pathlib import Path

import pytest

from tools import HANDLERS, reset_pool

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
    if argv and argv[0] == "grid":
        print('{{"domains": ["DEFEND", "HEAL"], "modes": ["TEACH", "WORK"], "shape": [2, 2], "total_cells": 4, "populated_cells": 3, "entries": 9, "expanded": true, "embedder": {{"kind": "onnx-explicit", "dimension": 384}}}}')
        raise SystemExit(0)
    if argv and argv[0] == "chain":
        print('{{"gaps": ["gap-a"], "control": {{"permuted": true}}, "drift": []}}')
        raise SystemExit(0)
    if argv and argv[0] == "classify":
        print("  HEAL        × MAINTAIN   0.940 ")
        print("  CONSTRUCT   × CREATE     0.921 ")
        print("Margin over runner-up: 0.019  effectively a tie")
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
