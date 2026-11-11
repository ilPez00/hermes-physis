import os
import sys
import tempfile
from pathlib import Path

import pytest

# Direct-import the modules for unit tests; test_plugin.py separately loads the
# package the way Hermes does.
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

# Reuse the shipped warm cache when present; otherwise one temp cache for the
# whole suite so only the first integration test pays the model cost.
_bundled_cache = Path(__file__).resolve().parents[1] / "models" / "cache"
os.environ.setdefault(
    "PHYSIS_CACHE_DIR",
    str(_bundled_cache) if _bundled_cache.is_dir() else tempfile.mkdtemp(prefix="physis-cache-"),
)


def pytest_configure(config):
    config.addinivalue_line(
        "markers", "security: catalog security hardening regression tests"
    )


@pytest.fixture(autouse=True)
def _isolated_hermes_home(tmp_path, monkeypatch):
    """Keep plugin state out of the real Hermes profile during tests."""
    monkeypatch.setenv("HERMES_HOME", str(tmp_path / "hermes-home"))
    yield


@pytest.fixture(autouse=True)
def _isolated_replant_store(tmp_path, monkeypatch):
    """Keep every test's replant state out of the real profile directory.

    The hardened plugin keeps recipes, candidates and learned registries in a
    fixed plugin-owned profile dir (operator-relocatable through
    PHYSIS_REPLANT_STORE, never through model arguments).
    """
    monkeypatch.setenv("PHYSIS_REPLANT_STORE", str(tmp_path / "replant-store"))
    yield


@pytest.fixture(autouse=True)
def _reset_engine_pool():
    # tools.py caches the discovered engines in a module global; drop it so one
    # test's fake engine never leaks into the next test's pool.
    import tools

    tools.reset_pool()
    yield
    tools.reset_pool()
