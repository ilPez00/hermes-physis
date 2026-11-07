#!/usr/bin/env python3
"""Trusted counter fixture for the bundled replant demo.

Not a model-facing tool and not production code: it exists so the
record -> replay loop can be demonstrated with a real action and a real,
independent check. The action mutates one file; the check re-reads it and
compares against the value the action saved, so a replayed recipe that did not
actually run would fail.

Replant passes each step's input as JSON on stdin; this fixture drains it and
accepts any object so the demo never fails on input shape.
"""

from __future__ import annotations

import json
import pathlib
import sys


def _read_int(path: pathlib.Path, default: int | None = None) -> int | None:
    try:
        return int(path.read_text().strip())
    except (OSError, ValueError):
        return default


def main() -> int:
    if len(sys.argv) != 3:
        print(json.dumps({"ok": False, "error": "usage: replant_fixture.py <increment|verify> <root>"}))
        return 2
    mode, root = sys.argv[1], pathlib.Path(sys.argv[2])
    root.mkdir(parents=True, exist_ok=True)
    try:  # drain stdin; replant supplies the step input as one JSON value
        json.load(sys.stdin)
    except (ValueError, OSError):
        pass

    counter = root / "counter"
    previous = root / "previous"

    if mode == "increment":
        before = _read_int(counter, 0) or 0
        previous.write_text(str(before))
        counter.write_text(str(before + 1))
        print(json.dumps({"ok": True, "count": before + 1}))
        return 0

    if mode == "verify":
        count = _read_int(counter)
        before = _read_int(previous)
        ok = (
            count is not None
            and before is not None
            and count == before + 1
            and not (root / "fail").exists()
        )
        print(json.dumps({"ok": ok, "count": count, "previous": before}))
        return 0 if ok else 1

    print(json.dumps({"ok": False, "error": f"unknown mode: {mode}"}))
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
