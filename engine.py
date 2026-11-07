"""Lightweight Physis engine wrapper.

The plugin deliberately depends on a small engine (physis-core or
physis-replant) instead of the full physis binary.  The exact command set of
those binaries is not assumed; capabilities are probed at runtime.
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Iterable


DEFAULT_TIMEOUT = 15
MAX_OUTPUT = 12000


class EngineError(Exception):
    """Raised when the engine cannot be found or used."""


class PhysisEngine:
    def __init__(self, binary: str | None = None, timeout: int = DEFAULT_TIMEOUT):
        self.timeout = max(1, int(timeout or DEFAULT_TIMEOUT))
        self.binary = self._resolve_binary(binary)
        self._capabilities: set[str] | None = None

    @staticmethod
    def _resolve_binary(binary: str | None) -> str:
        if binary:
            if os.path.isfile(binary) and os.access(binary, os.X_OK):
                return binary
            found = shutil.which(binary)
            if found:
                return found
            raise EngineError(f"configured engine binary not found: {binary}")

        for candidate in ("physis-core", "physis-replant", "physis"):
            found = shutil.which(candidate)
            if found:
                return found
        env_bin = os.environ.get("PHYSIS_BIN")
        if env_bin and (os.path.isfile(env_bin) and os.access(env_bin, os.X_OK)):
            return env_bin
        raise EngineError(
            "no lightweight Physis engine found; install physis-core or "
            "physis-replant, or set PHYSIS_BIN"
        )

    def capabilities(self) -> set[str]:
        if self._capabilities is not None:
            return self._capabilities
        try:
            proc = subprocess.run(
                [self.binary, "--help"],
                capture_output=True,
                text=True,
                timeout=min(self.timeout, 5),
                check=False,
            )
            text = (proc.stdout or "") + "\n" + (proc.stderr or "")
        except (subprocess.TimeoutExpired, OSError):
            text = ""
        # Match command names in clap-style help: "    observe " or "observe,"
        found = set(re.findall(r"(?m)^\s{2,}([a-z][a-z0-9_-]*)\b", text))
        # Also accept words immediately followed by a description after two spaces.
        found.update(re.findall(r"(?m)^\s{2,}([a-z][a-z0-9_-]*)\s{2,}", text))
        self._capabilities = found
        return found

    def supports(self, command: str) -> bool:
        caps = self.capabilities()
        if not caps:
            # If help parsing failed, be optimistic but let execution decide.
            return True
        return command in caps

    def run(self, args: Iterable[str], cwd: str | None = None, timeout: int | None = None) -> dict[str, Any]:
        argv = [self.binary, *args]
        effective_timeout = self.timeout if timeout is None else max(1, int(timeout))
        try:
            proc = subprocess.run(
                argv,
                cwd=cwd,
                capture_output=True,
                text=True,
                timeout=effective_timeout,
                check=False,
            )
        except subprocess.TimeoutExpired as exc:
            return {
                "ok": False,
                "binary": self.binary,
                "argv": argv,
                "returncode": None,
                "stdout": _truncate(exc.stdout or ""),
                "stderr": f"timed out after {effective_timeout}s",
                "timed_out": True,
            }
        except OSError as exc:
            return {
                "ok": False,
                "binary": self.binary,
                "argv": argv,
                "returncode": None,
                "stdout": "",
                "stderr": str(exc),
                "timed_out": False,
            }

        stdout = _truncate(proc.stdout or "")
        stderr = _truncate(proc.stderr or "")
        return {
            "ok": proc.returncode == 0,
            "binary": self.binary,
            "argv": argv,
            "returncode": proc.returncode,
            "stdout": stdout,
            "stderr": stderr,
            "timed_out": False,
        }

    def info(self) -> dict[str, Any]:
        return {
            "binary": self.binary,
            "timeout": self.timeout,
            "capabilities": sorted(self.capabilities()),
        }


def _truncate(text: str) -> str:
    if len(text) <= MAX_OUTPUT:
        return text
    return text[:MAX_OUTPUT] + f"\n... truncated {len(text) - MAX_OUTPUT} bytes"
