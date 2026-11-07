import os
import sys
import tempfile
from pathlib import Path

# Direct-import the modules for unit tests; test_plugin.py separately loads the
# package the way Hermes does.
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

# One embedding cache for the whole suite: the first integration test pays the
# one-time model cost, every later test reuses it.
os.environ.setdefault("PHYSIS_CACHE_DIR", tempfile.mkdtemp(prefix="physis-cache-"))
