import os
import sys
import tempfile
from pathlib import Path

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
