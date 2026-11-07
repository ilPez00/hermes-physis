"""Lightweight Physis engine wrapper.

The plugin deliberately depends on small engines (physis-core or
physis-replant) instead of the full physis binary.  Several engines may be
available at once; the plugin discovers all of them and routes each command to
an engine that advertises it.
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

CANDIDATES = ("physis-core", "physis-replant", "replant", "physis", "physis-next")

# Bundled engines shipped inside this plugin (bin/ directory).
PLUGIN_DIR = Path(__file__).resolve().parent
BUNDLED_BIN_DIR = PLUGIN_DIR / "bin"
BUNDLED_CANDIDATES = (
    BUNDLED_BIN_DIR / "physis-core",
    BUNDLED_BIN_DIR / "replant",
)


class EngineError(Exception):
    """Raised when no engine can be found or used."""


class PhysisEngine:
    def __init__(self, binary: str, timeout: int = DEFAULT_TIMEOUT):
        self.timeout = max(1, int(timeout or DEFAULT_TIMEOUT))
        self.binary = self.resolve_binary(binary)
        self._capabilities: set[str] | None = None

    @staticmethod
    def resolve_binary(binary: str | None) -> str:
        if binary:
            if os.path.isfile(binary) and os.access(binary, os.X_OK):
                return binary
            found = shutil.which(binary)
            if found:
                return found
            raise EngineError(f"configured engine binary not found: {binary}")
        for candidate in CANDIDATES:
            found = shutil.which(candidate)
            if found:
                return found
        env_bin = os.environ.get("PHYSIS_BIN")
        if env_bin and os.path.isfile(env_bin) and os.access(env_bin, os.X_OK):
            return env_bin
        raise EngineError(
            "no Phasis engine found; install physis-core or physis-replant, "
            "or set PHYSIS_BIN"
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
        found = set(re.findall(r"(?m)^\s{2,}([a-z][a-z0-9_-]*)\b", text))
        found.update(re.findall(r"(?m)^\s{2,}([a-z][a-z0-9_-]*)\s{2,}", text))
        # replant emits --help as JSON: {"usage":"replant <run|record|...> [options]..."}
        try:
            import json as _json
            payload = _json.loads(text)
            usage = str(payload.get("usage", "") or payload.get("help", ""))
            found.update(re.findall(r"\b(run|record|show|history|stats|approve|reject|undo|serve|mcp)\b", usage))
        except (ValueError, TypeError):
            pass
        self._capabilities = found
        return found

    def supports(self, command: str) -> bool:
        caps = self.capabilities()
        if not caps:
            return True
        return command in caps

    def run(self, args: Iterable[str], cwd: str | None = None, timeout: int | None = None, stdin: str | None = None) -> dict[str, Any]:
        argv = [self.binary, *args]
        effective_timeout = self.timeout if timeout is None else max(1, int(timeout))
        # If cwd doesn't exist, fall back to the current directory.
        if cwd is not None and not Path(cwd).is_dir():
            cwd = None
        try:
            proc = subprocess.run(
                argv,
                cwd=cwd,
                capture_output=True,
                text=True,
                timeout=effective_timeout,
                check=False,
                input=stdin,
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

        return {
            "ok": proc.returncode == 0,
            "binary": self.binary,
            "argv": argv,
            "returncode": proc.returncode,
            "stdout": _truncate(proc.stdout or ""),
            "stderr": _truncate(proc.stderr or ""),
            "timed_out": False,
        }

    def info(self) -> dict[str, Any]:
        return {
            "binary": self.binary,
            "timeout": self.timeout,
            "capabilities": sorted(self.capabilities()),
        }


class EnginePool:
    """A collection of engines, routed by advertised command."""

    def __init__(self, engines: list[PhysisEngine]):
        if not engines:
            raise EngineError("no Physis engines available")
        self.engines = engines
        self.timeout = engines[0].timeout

    @property
    def binary(self) -> str:
        return self.engines[0].binary

    def capabilities(self) -> set[str]:
        result: set[str] = set()
        for engine in self.engines:
            result |= engine.capabilities()
        return result

    def supports(self, command: str) -> bool:
        caps = self.capabilities()
        if not caps:
            return True
        return command in caps

    def engine_for(self, command: str) -> PhysisEngine:
        for engine in self.engines:
            if engine.supports(command):
                return engine
        return self.engines[0]

    def run(self, args: Iterable[str], cwd: str | None = None, timeout: int | None = None, stdin: str | None = None) -> dict[str, Any]:
        argv = list(args)
        command = argv[0] if argv else ""
        engine = self.engine_for(command)
        return engine.run(argv, cwd=cwd, timeout=timeout, stdin=stdin)

    def info(self) -> dict[str, Any]:
        return {
            "engines": [engine.info() for engine in self.engines],
            "capabilities": sorted(self.capabilities()),
        }


def discover_engines(binary: Any = None, timeout: int = DEFAULT_TIMEOUT) -> EnginePool:
    binaries: list[str] = []
    if isinstance(binary, str) and binary.strip():
        if "," in binary:
            binaries = [item.strip() for item in binary.split(",") if item.strip()]
        else:
            binaries = [binary.strip()]
    elif isinstance(binary, (list, tuple)):
        binaries = [str(item).strip() for item in binary if str(item).strip()]
    elif binary is None:
        env_bins = os.environ.get("PHYSIS_BINS") or os.environ.get("PHYSIS_BIN")
        if env_bins:
            binaries = [item.strip() for item in env_bins.split(",") if item.strip()]
        else:
            # Prefer binaries bundled with the plugin, if present.
            for bundled in BUNDLED_CANDIDATES:
                if bundled.is_file() and os.access(bundled, os.X_OK):
                    binaries.append(str(bundled))
            # Also check arch-specific subdirs (bin/linux-x86_64/...).
            import platform as _platform

            machine = _platform.machine().lower()
            system = _platform.system().lower()
            arch = "x86_64" if machine in {"x86_64", "amd64"} else machine
            os_name = "linux" if system == "linux" else system
            for name in ("physis-core", "replant"):
                arch_bundled = (
                    Path(__file__).resolve().parent / "bin" / f"{os_name}-{arch}" / name
                )
                if (
                    arch_bundled.is_file()
                    and os.access(arch_bundled, os.X_OK)
                    and str(arch_bundled) not in binaries
                ):
                    binaries.append(str(arch_bundled))
            for candidate in CANDIDATES:
                found = shutil.which(candidate)
                if found and found not in binaries:
                    binaries.append(found)

    if not binaries:
        # Fall back to a single configured/available binary; may raise.
        binaries = [PhysisEngine.resolve_binary(None)]

    engines: list[PhysisEngine] = []
    seen: set[str] = set()
    for item in binaries:
        try:
            resolved = PhysisEngine.resolve_binary(item)
        except EngineError:
            continue
        if resolved in seen:
            continue
        seen.add(resolved)
        engines.append(PhysisEngine(resolved, timeout=timeout))

    if not engines:
        raise EngineError(
            "no Physis engines found; install physis-core or physis-replant, "
            "or set PHYSIS_BIN/PHYSIS_BINS"
        )
    return EnginePool(engines)


def _truncate(text: str) -> str:
    if len(text) <= MAX_OUTPUT:
        return text
    return text[:MAX_OUTPUT] + f"\n... truncated {len(text) - MAX_OUTPUT} bytes"
