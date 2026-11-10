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


def test_context_code_mode_uses_graph_and_autoscans(isolated_store, repo):
    result = HANDLERS["physis_context"](
        {"path": str(repo), "task": "rotate an expired token", "budget": 300, "timeout": 60}
    )
    assert result["ok"] is True
    assert result["mode"] == "code"
    assert result["auto_indexed"] is True
    assert result["results"] >= 1
    assert "auth.py" in result["stdout"]


def test_context_docs_mode_reports_compression(isolated_store, tmp_path):
    corpus = tmp_path / "docs"
    corpus.mkdir()
    (corpus / "runbook.md").write_text("# Runbook\n\nIf pressure exceeds 2.6 bar, stop the pump.\n")
    (corpus / "policy.txt").write_text("Retention: vibration records are kept for 5 years.\n")
    result = HANDLERS["physis_context"](
        {"path": str(corpus), "task": "valve pressure", "budget": 200, "mode": "docs", "timeout": 60}
    )
    assert result["ok"] is True
    assert result["mode"] == "docs"
    assert result["source_documents"] == 2
    assert result["physis_tokens"] <= result["baseline_tokens"]


def test_classify_and_direction(isolated_store, repo):
    before = repo / "before.txt"
    after = repo / "after.txt"
    before.write_text("the pump failed because the seal wore out again and again")
    after.write_text("seal worn out")

    classified = HANDLERS["physis_classify"]({"text": "def rotate_token(token): pass", "timeout": 60})
    assert classified["ok"] is True
    assert classified["cells"]

    diff = HANDLERS["physis_diff"](
        {"before": str(before), "after": str(after), "want": "shorter", "timeout": 60}
    )
    assert diff["ok"] is True
    assert "moved" in diff


def test_chain_runs_on_real_corpus(isolated_store, repo):
    result = HANDLERS["physis_chain"](
        {"path": str(repo), "task": "authentication", "budget": 300, "timeout": 90}
    )
    assert result["ok"] is True
    assert result["argv"][1] == "chain"


def test_grid_is_expanded_and_semantic(isolated_store):
    result = HANDLERS["physis_grid"]({"timeout": 120})
    assert result["ok"] is True
    assert result["shape"][0] >= 7 and result["shape"][1] >= 16
    assert result["expanded"] is True
    assert result["populated_cells"] > 70
    assert result["embedder"]["kind"] != "random-projection", result["embedder"]


def test_context_docs_includes_code(isolated_store, repo):
    result = HANDLERS["physis_context"](
        {"path": str(repo), "task": "rotate an expired token", "budget": 300, "mode": "docs", "timeout": 120}
    )
    assert result["ok"] is True
    assert result["source_documents"] >= 3  # .py files are corpus now, not just docs
    assert result["physis_tokens"] >= 32


def test_classify_uses_semantic_embedder(isolated_store):
    result = HANDLERS["physis_classify"]({"text": "revoke the leaked token", "timeout": 120})
    assert result["ok"] is True
    assert "onnx" in result["stdout"], result["stdout"]
    assert result["embedder_kind"].startswith("onnx")
    assert result["grid_shape"] == [8, 20]


def test_ingest_and_observed_roundtrip(isolated_store, repo):
    ingested = HANDLERS["physis_ingest"]({"path": str(repo), "source": "fs", "max": 20, "timeout": 60})
    assert ingested["ok"] is True
    assert ingested["appended"] >= 1
    observed = HANDLERS["physis_observed"]({"limit": 5, "timeout": 60})
    assert observed["ok"] is True
    assert observed["count"] >= 1
    assert observed["observations"][0]["source"] == "fs"


def test_classify_auto_expands_on_tie(isolated_store):
    result = HANDLERS["physis_classify"](
        {"text": "rotate an expired authentication token", "timeout": 120}
    )
    assert result["ok"] is True
    assert result.get("created_entry"), result["stdout"]
    assert result["created_cell"]


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
    assert result["provider_calls"] == 0


def test_no_recalled_nodes_hint(isolated_store, repo):
    result = HANDLERS["physis_query"]({"path": str(repo), "question": "anything", "timeout": 60})
    assert result["ok"] is True
    assert "hint" in result
