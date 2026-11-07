import json
import stat
import textwrap
from pathlib import Path

import pytest

from engine import EngineError, PhysisEngine, discover_engines


def make_binary(tmp_path: Path, help_text: str, behavior: str = "") -> str:
    path = tmp_path / "fake-engine"
    script = textwrap.dedent(f"""\
    #!/usr/bin/env python3
    import sys
    argv = sys.argv[1:]
    if argv and argv[0] == "--help":
        print({help_text!r})
        raise SystemExit(0)
    {behavior}
    print("stdout:" + " ".join(argv))
    raise SystemExit(0)
    """)
    path.write_text(script)
    path.chmod(path.stat().st_mode | stat.S_IEXEC)
    return str(path)


def test_resolve_binary_prefers_configured(tmp_path):
    binary = make_binary(tmp_path, "  scan  Scan\n")
    engine = PhysisEngine(binary=binary)
    assert engine.binary == binary


def test_capabilities_parse_physis_core_help(tmp_path):
    help_text = "Usage: physis-core <COMMAND>\n\nCommands:\n  scan       Scan a directory\n  search     Search nodes\n  node-search  Budget search\n"
    engine = PhysisEngine(binary=make_binary(tmp_path, help_text))
    assert {"scan", "search", "node-search"} <= engine.capabilities()
    assert not engine.supports("ground")


def test_capabilities_parse_replant_json_help(tmp_path):
    usage = {"usage": "replant <run|record|show|history|stats|approve|reject|undo|serve|mcp> [options]"}
    engine = PhysisEngine(binary=make_binary(tmp_path, json.dumps(usage)))
    assert {"run", "record", "stats", "show"} <= engine.capabilities()
    assert not engine.supports("scan")


def test_run_passes_env(tmp_path):
    binary = make_binary(
        tmp_path,
        "  scan  Scan\n",
        'import os; print("DIR=" + os.environ.get("PHYSIS_CORE_DIR", ""))',
    )
    engine = PhysisEngine(binary=binary, timeout=5)
    result = engine.run(["scan", "."], env={"PHYSIS_CORE_DIR": "/tmp/store"})
    assert result["ok"] is True
    assert "DIR=/tmp/store" in result["stdout"]


def test_run_success_and_truncate(tmp_path):
    binary = make_binary(tmp_path, "  scan  Scan\n", 'print("x" * 20000)')
    engine = PhysisEngine(binary=binary, timeout=5)
    result = engine.run(["scan", "."])
    assert result["ok"] is True
    assert len(result["stdout"]) <= 16100


def test_run_timeout(tmp_path):
    binary = make_binary(tmp_path, "  scan  Scan\n", "import time; time.sleep(5)")
    engine = PhysisEngine(binary=binary, timeout=1)
    result = engine.run(["scan", "."], timeout=1)
    assert result["ok"] is False
    assert result["timed_out"] is True


def test_bundled_model_dir_is_injected(tmp_path, monkeypatch):
    monkeypatch.delenv("PHYSIS_MODEL_DIR", raising=False)
    binary = make_binary(
        tmp_path,
        "  scan  Scan\n",
        'import os; print("MODEL=" + os.environ.get("PHYSIS_MODEL_DIR", ""))',
    )
    engine = PhysisEngine(binary=binary, timeout=5)
    result = engine.run(["scan", "."])
    assert result["ok"] is True
    assert "models/bge-small" in result["stdout"].replace("\\", "/")


def test_missing_binary_raises():
    with pytest.raises(EngineError):
        PhysisEngine(binary="/definitely/not/here")


def test_discover_dedupes(tmp_path):
    binary = make_binary(tmp_path, "  scan  Scan\n")
    pool = discover_engines(binary=f"{binary},{binary}", timeout=5)
    assert len(pool.engines) == 1
    assert binary == pool.binary
