"""Discover and run the small Physis engines bundled with this plugin.

Two binaries are shipped in ``bin/``:

* ``physis-core`` - deterministic ontology/coherence engine (scan, search,
  node-search, classify, snapshot, ...).
* ``replant`` - verified tool-recipe replay (run, record, stats, show, ...).

The plugin never guesses a command: it reads each engine's own ``--help``
output and only routes a command to an engine that advertises it. That keeps
the tool layer honest when one of the engines is upgraded or replaced.
"""

from __future__ import annotations

import json
import os
import platform
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Iterable

DEFAULT_TIMEOUT = 20
MAX_OUTPUT = 16000

PLUGIN_DIR = Path(__file__).resolve().parent
BUNDLED_BIN_DIR = PLUGIN_DIR / "bin"

# Names looked up on PATH when no bundled binary is present.
CANDIDATES = ("physis-core", "replant", "physis-replant", "physis", "physis-next")

_REPLANT_COMMANDS = re.compile(
    r"\b(run|record|show|history|stats|approve|reject|undo|serve|mcp)\b"
)


class EngineError(Exception):
    """Raised when no engine can be found or a configured path is unusable."""


def _is_executable(path: Path) -> bool:
    return path.is_file() and os.access(path, os.X_OK)


class PhysisEngine:
    """One engine binary plus its self-reported command surface."""

    def __init__(self, binary: str, timeout: int = DEFAULT_TIMEOUT):
        self.timeout = max(1, int(timeout or DEFAULT_TIMEOUT))
        self.binary = self.resolve_binary(binary)
        self._capabilities: set[str] | None = None

    @staticmethod
    def resolve_binary(binary: str | None) -> str:
        if binary:
            raw = os.path.expanduser(str(binary).strip())
            if _is_executable(Path(raw)):
                return raw
            found = shutil.which(raw)
            if found:
                return found
            raise EngineError(f"configured engine binary not found or not executable: {binary}")
        for candidate in CANDIDATES:
            found = shutil.which(candidate)
            if found:
                return found
        env_bin = os.environ.get("PHYSIS_BIN")
        if env_bin and _is_executable(Path(env_bin)):
            return env_bin
        raise EngineError(
            "no Physis engine found; the plugin ships bin/physis-core and bin/replant, "
            "or set PHYSIS_BIN / plugins.entries.physis.settings.engine_bin"
        )

    def capabilities(self) -> set[str]:
        """Commands the engine advertises in ``--help`` (cached)."""
        if self._capabilities is not None:
            return self._capabilities
        try:
            proc = subprocess.run(
                [self.binary, "--help"],
                capture_output=True,
                text=True,
                timeout=min(self.timeout, 10),
                check=False,
            )
            text = (proc.stdout or "") + "\n" + (proc.stderr or "")
        except (subprocess.TimeoutExpired, OSError):
            text = ""

        found: set[str] = set()
        # physis-core: "Commands:\n  scan       Scan a directory..."
        found.update(re.findall(r"(?m)^\s{2}([a-z][a-z0-9_-]*)\s{2,}\S", text))
        # replant: --help is one JSON object whose "usage" lists <run|record|...>
        try:
            payload = json.loads(text.strip())
        except (ValueError, TypeError):
            payload = None
        if isinstance(payload, dict):
            usage = " ".join(str(payload.get(key, "")) for key in ("usage", "help"))
            found.update(_REPLANT_COMMANDS.findall(usage))
        self._capabilities = found
        return found

    def supports(self, command: str) -> bool:
        caps = self.capabilities()
        # An engine that tells us nothing is tried rather than ruled out.
        return True if not caps else command in caps

    def version(self) -> str:
        try:
            proc = subprocess.run(
                [self.binary, "--version"],
                capture_output=True, text=True, timeout=5, check=False,
            )
        except (subprocess.TimeoutExpired, OSError):
            return ""
        text = (proc.stdout or proc.stderr or "").strip()
        return text.splitlines()[0][:120] if proc.returncode == 0 and text else ""

    def run(
        self,
        args: Iterable[str],
        cwd: str | None = None,
        timeout: int | None = None,
        stdin: str | None = None,
        env: dict[str, str] | None = None,
    ) -> dict[str, Any]:
        argv = [self.binary, *[str(a) for a in args]]
        effective_timeout = self.timeout if timeout is None else max(1, int(timeout))
        workdir = cwd if cwd and Path(cwd).is_dir() else None
        run_env = None
        if env:
            run_env = {**os.environ, **{str(k): str(v) for k, v in env.items()}}
        try:
            proc = subprocess.run(
                argv,
                cwd=workdir,
                capture_output=True,
                text=True,
                timeout=effective_timeout,
                check=False,
                input=stdin,
                env=run_env,
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
            "version": self.version(),
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
        return any(engine.supports(command) for engine in self.engines)

    def engine_for(self, command: str, preferred: str | None = None) -> PhysisEngine:
        if preferred:
            for engine in self.engines:
                if preferred in Path(engine.binary).name and engine.supports(command):
                    return engine
        for engine in self.engines:
            if engine.supports(command):
                return engine
        return self.engines[0]

    def run(
        self,
        args: Iterable[str],
        cwd: str | None = None,
        timeout: int | None = None,
        stdin: str | None = None,
        env: dict[str, str] | None = None,
        engine: str | None = None,
    ) -> dict[str, Any]:
        argv = list(args)
        command = argv[0] if argv else ""
        return self.engine_for(command, preferred=engine).run(
            argv, cwd=cwd, timeout=timeout, stdin=stdin, env=env
        )

    def info(self) -> dict[str, Any]:
        return {
            "engines": [engine.info() for engine in self.engines],
            "capabilities": sorted(self.capabilities()),
        }


def _arch_dir_names() -> tuple[str, str]:
    machine = platform.machine().lower()
    arch = "x86_64" if machine in {"x86_64", "amd64"} else machine
    system = platform.system().lower()
    os_name = "linux" if system == "linux" else system
    return os_name, arch


def _bundled_binaries() -> list[str]:
    """Bundled binaries, preferring the flat ``bin/`` layout (deduplicated)."""
    found: list[str] = []
    for name in ("physis-core", "replant"):
        flat = BUNDLED_BIN_DIR / name
        if _is_executable(flat):
            found.append(str(flat))
    os_name, arch = _arch_dir_names()
    for name in ("physis-core", "replant"):
        arch_path = BUNDLED_BIN_DIR / f"{os_name}-{arch}" / name
        if _is_executable(arch_path):
            resolved = str(arch_path.resolve())
            if resolved not in {str(Path(item).resolve()) for item in found}:
                found.append(str(arch_path))
    return found


def discover_engines(binary: Any = None, timeout: int = DEFAULT_TIMEOUT) -> EnginePool:
    binaries: list[str] = []
    if isinstance(binary, str) and binary.strip():
        # A comma-separated list is accepted for operators who keep several
        # engines in different locations.
        binaries = [item.strip() for item in binary.split(",") if item.strip()]
    elif isinstance(binary, (list, tuple)):
        binaries = [str(item).strip() for item in binary if str(item).strip()]
    else:
        env_bins = os.environ.get("PHYSIS_BINS") or os.environ.get("PHYSIS_BIN")
        if env_bins:
            binaries = [item.strip() for item in env_bins.split(",") if item.strip()]
        else:
            # Bundled engines are the supported pair. Only look at PATH when
            # the bundle is missing, so a stray `physis`/`physis-next` binary
            # cannot silently change the tool surface.
            binaries = _bundled_binaries()
            if not binaries:
                for candidate in CANDIDATES:
                    found = shutil.which(candidate)
                    if found:
                        binaries.append(found)

    engines: list[PhysisEngine] = []
    seen: set[str] = set()
    errors: list[str] = []
    for item in binaries:
        try:
            resolved = str(Path(PhysisEngine.resolve_binary(item)).resolve())
        except EngineError as exc:
            errors.append(str(exc))
            continue
        if resolved in seen:
            continue
        seen.add(resolved)
        engines.append(PhysisEngine(resolved, timeout=timeout))

    if not engines:
        # An explicit operator path must fail with that path in the message,
        # not a generic "nothing found".
        if errors and (isinstance(binary, str) and binary.strip() or binary is not None):
            raise EngineError(errors[0])
        raise EngineError(
            "no Physis engines found; expected bundled bin/physis-core and bin/replant, "
            "or set PHYSIS_BIN/PHYSIS_BINS"
        )
    return EnginePool(engines)


def _truncate(text: str) -> str:
    text = text or ""
    if len(text) <= MAX_OUTPUT:
        return text
    return text[:MAX_OUTPUT] + f"\n... truncated {len(text) - MAX_OUTPUT} bytes"
