import os
import stat
import textwrap
from pathlib import Path

import pytest

from engine import PhysisEngine, EngineError


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
    binary = make_binary(tmp_path, "observe context")
    engine = PhysisEngine(binary=binary)
    assert engine.binary == binary


def test_capabilities_parse_help(tmp_path):
    binary = make_binary(tmp_path, "  observe  Scan\n  context  Compile\n")
    engine = PhysisEngine(binary=binary)
    assert engine.supports("observe")
    assert engine.supports("context")
    assert not engine.supports("diff")


def test_run_success_and_truncate(tmp_path):
    binary = make_binary(tmp_path, "observe", 'print("x" * 20000)')
    engine = PhysisEngine(binary=binary, timeout=5)
    result = engine.run(["observe", "."])
    assert result["ok"] is True
    assert len(result["stdout"]) <= 12100


def test_run_timeout(tmp_path):
    binary = make_binary(tmp_path, "observe", "import time; time.sleep(5)")
    engine = PhysisEngine(binary=binary, timeout=1)
    result = engine.run(["observe", "."], timeout=1)
    assert result["ok"] is False
    assert result["timed_out"] is True


def test_missing_binary_raises():
    with pytest.raises(EngineError):
        PhysisEngine(binary="/definitely/not/here")
