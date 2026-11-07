"""Python implementation of the replant memo (cost crystallization)."""

from __future__ import annotations

import hashlib
import json
import time
from pathlib import Path
from typing import Any


class ReplantMemo:
    def __init__(self, root: str | Path = "."):
        self.root = Path(root)
        self.store_path = self.root / ".physis" / "replant.json"
        self._data: dict[str, Any] = {"entries": {}}
        self._load()

    def _load(self) -> None:
        try:
            if self.store_path.is_file():
                self._data = json.loads(self.store_path.read_text())
        except (OSError, json.JSONDecodeError):
            self._data = {"entries": {}}
        self._data.setdefault("entries", {})

    def _save(self) -> None:
        self.store_path.parent.mkdir(parents=True, exist_ok=True)
        self.store_path.write_text(json.dumps(self._data, indent=2, sort_keys=True))

    @staticmethod
    def key_for(value: str) -> str:
        return hashlib.sha256(value.encode("utf-8")).hexdigest()

    def lookup(self, value: str) -> dict[str, Any] | None:
        return self._data["entries"].get(self.key_for(value))

    def record(self, value: str, result: Any, elapsed_ms: float = 0.0) -> dict[str, Any]:
        entry = {
            "input": value,
            "result": result,
            "elapsed_ms": float(elapsed_ms),
            "created_at": time.time(),
        }
        self._data["entries"][self.key_for(value)] = entry
        self._save()
        return entry

    def run(self, value: str, result: Any = None, elapsed_ms: float = 0.0) -> dict[str, Any]:
        started = time.perf_counter()
        existing = self.lookup(value)
        if existing is not None:
            return {
                "tier": "T0a",
                "cache_hit": True,
                "result": existing["result"],
                "saved_ms": existing.get("elapsed_ms", 0.0),
                "key": self.key_for(value),
            }
        if result is None:
            result = {"input": value, "computed": True}
        if elapsed_ms <= 0:
            elapsed_ms = (time.perf_counter() - started) * 1000.0
        entry = self.record(value, result, elapsed_ms)
        return {
            "tier": "T2",
            "cache_hit": False,
            "result": entry["result"],
            "elapsed_ms": entry["elapsed_ms"],
            "key": self.key_for(value),
        }

    def stats(self) -> dict[str, Any]:
        entries = self._data["entries"]
        return {
            "entries": len(entries),
            "store_path": str(self.store_path),
        }

    def demo(self, iterations: int = 10) -> dict[str, Any]:
        value = "replant demo input"
        runs = []
        for _ in range(max(1, iterations)):
            runs.append(self.run(value))
        hits = sum(1 for run in runs if run["cache_hit"])
        return {
            "iterations": len(runs),
            "hits": hits,
            "misses": len(runs) - hits,
            "effectiveness_pct": round(100.0 * hits / len(runs), 1),
            "runs": runs,
        }
