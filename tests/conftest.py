import sys
from pathlib import Path

# Direct-import the modules for unit tests; test_plugin.py separately loads the
# package the way Hermes does.
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
