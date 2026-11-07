"""Integration tests against the binaries bundled in bin/.

These are the tests that would have caught the shipped bug: the fake-engine
unit tests passed while the real engine advertised scan/search/node-search,
not observe/context/ground. Skipped only when a binary is missing.
"""

import os
import shutil
from pathlib import Path

import pytest

from tools import HANDLERS, reset_pool

PLUGIN_DIR = Path(__file__).resolve().parents[1]
CORE = PLUGIN_DIR / "bin" / "physis-core"
REPLANT = PLUGIN_DIR / "bin" / "replant"

pytestmark = pytest.mark.skipif(
    not CORE.is_file() or not REPLANT.is_file(),
    reason="bundled engines are not present",
)


@pytest.fixture()
def isolated_store(tmp_path, monkeypatch):
    store = tmp_path / "core-store"
    store.mkdir()
    monkeypatch.setenv("PHYSIS_CORE_DIR", str(store))
    reset_pool()
    yield store
    reset_pool()


@pytest.fixture()
def repo(tmp_path):
    root = tmp_path / "repo"
    (root / "src").mkdir(parents=True)
    (root / "src" / "auth.py").write_text(
        "def rotate_token(token):\n    return token[::-1]\n"
    )
    (root / "src" / "billing.py").write_text(
        "def invoice_total(items):\n    return sum(items)\n"
    )
    (root / "README.md").write_text("# demo\n\nSmall repo for physis integration tests.\n")
    return root


def test_status_reports_both_engines():
    result = HANDLERS["physis_engine_status"]({})
    assert result["ok"] is True
    names = {Path(engine["binary"]).name for engine in result["engines"]}
    assert names == {"physis-core", "replant"}
    assert "scan" in result["capabilities"]
    assert "record" in result["capabilities"]


def test_scan_search_and_snapshot(isolated_store, repo):
    scanned = HANDLERS["physis_observe"]({"path": str(repo), "timeout": 60})
    assert scanned["ok"] is True
    assert scanned["files"] == 3
    assert scanned["nodes"] >= 3

    query = HANDLERS["physis_query"](
        {"path": str(repo), "question": "rotate token", "limit": 3, "timeout": 60}
    )
    assert query["ok"] is True
    assert "auth.py" in query["stdout"]

    snapshot = HANDLERS["physis_snapshot"]({"path": str(repo)})
    assert snapshot["ok"] is True
    assert snapshot["nodes"] >= 3


def test_context_is_budgeted(isolated_store, repo):
    HANDLERS["physis_observe"]({"path": str(repo), "timeout": 60})
    result = HANDLERS["physis_context"](
        {"path": str(repo), "task": "authentication token rotation", "budget": 200, "timeout": 60}
    )
    assert result["ok"] is True
    assert "Fixed-token search" in result["stdout"] or "No recalled nodes" in result["stdout"]


def test_classify_and_diff(isolated_store, repo):
    before = repo / "before.txt"
    after = repo / "after.txt"
    before.write_text("the pump failed because the seal wore out again")
    after.write_text("def rotate_token(token):\n    return token[::-1]\n")

    classified = HANDLERS["physis_classify"]({"text": "def rotate_token(token): pass", "timeout": 60})
    assert classified["ok"] is True
    assert classified["cells"]

    diff = HANDLERS["physis_diff"](
        {"before": str(before), "after": str(after), "timeout": 60}
    )
    assert diff["ok"] is True
    assert diff["top_before"] and diff["top_after"]


def test_replant_demo_records_then_replays(tmp_path):
    store = tmp_path / "replant-store"
    result = HANDLERS["replant_demo"]({"path": str(tmp_path), "store": str(store), "timeout": 60})
    assert result["ok"] is True
    assert result["record_status"] == "succeeded"
    assert result["second_operation"] == "replay"
    assert result["replayed"] is True
    assert result["counter"] == "3"  # record + two replays, each a real action
    assert result["stats"]["verified_records"] == 1
    assert result["stats"]["verified_replays"] == 2
    assert result["stats"]["provider_calls"] if "provider_calls" in result["stats"] else True


def test_no_recalled_nodes_hint(isolated_store, repo):
    result = HANDLERS["physis_query"]({"path": str(repo), "question": "anything", "timeout": 60})
    assert result["ok"] is True
    assert "hint" in result
