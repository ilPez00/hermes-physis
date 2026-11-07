import os
import stat
import textwrap
from pathlib import Path

from tools import HANDLERS


def make_binary(tmp_path: Path, supported: set[str]) -> str:
    help_lines = "".join(f"  {cmd}  desc\\n" for cmd in supported)
    path = tmp_path / "fake-engine"
    script = textwrap.dedent(f"""\
    #!/usr/bin/env python3
    import sys
    argv = sys.argv[1:]
    if argv and argv[0] == "--help":
        print({help_lines!r})
        raise SystemExit(0)
    print("ran:" + " ".join(argv))
    raise SystemExit(0)
    """)
    path.write_text(script)
    path.chmod(path.stat().st_mode | stat.S_IEXEC)
    return str(path)


def test_engine_status_ok(tmp_path):
    binary = make_binary(tmp_path, {"observe", "context"})
    result = HANDLERS["physis_engine_status"]({"engine_bin": binary})
    assert result["ok"] is True
    assert binary == result["engines"][0]["binary"]


def test_observe_unsupported(tmp_path):
    binary = make_binary(tmp_path, {"context"})
    result = HANDLERS["physis_observe"]({"engine_bin": binary, "path": "."})
    assert result["ok"] is False
    assert "does not advertise" in result["error"] or "no available engine" in result["error"]


def test_context_runs(tmp_path):
    binary = make_binary(tmp_path, {"context"})
    result = HANDLERS["physis_context"]({"engine_bin": binary, "path": str(tmp_path), "budget": 200})
    assert result["ok"] is True
    assert "ran:context" in result["stdout"]


def test_why_requires_object(tmp_path):
    binary = make_binary(tmp_path, {"why"})
    result = HANDLERS["physis_why"]({"engine_bin": binary, "path": str(tmp_path)})
    assert result["ok"] is False
    assert "object is required" in result["error"]


def test_diff_default_refs(tmp_path):
    binary = make_binary(tmp_path, {"diff"})
    result = HANDLERS["physis_diff"]({"engine_bin": binary, "path": str(tmp_path)})
    assert result["ok"] is True
    assert "HEAD~1..HEAD" in result["stdout"]
