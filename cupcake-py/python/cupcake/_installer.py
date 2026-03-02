"""OPA binary installer for Cupcake Python bindings.

Handles automatic download and SHA-256 verification of the OPA binary
required for policy compilation.  This is a direct port of
``cupcake-ts/installer.ts``.

OPA Version: v0.70.0
"""

from __future__ import annotations

import asyncio
import hashlib
import logging
import os
import platform
import shutil
import stat
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.request import urlopen, Request

logger = logging.getLogger("cupcake.installer")

OPA_VERSION = "v0.70.0"
_OPA_BASE_URL = f"https://github.com/open-policy-agent/opa/releases/download/{OPA_VERSION}"

# Platform-specific binary mapping with SHA-256 checksums.
# Keys are ``{sys.platform}-{machine}`` after normalisation.
_OPA_BINARIES: dict[str, dict[str, str | float]] = {
    "darwin-x86_64": {
        "binary": "opa_darwin_amd64",
        "sha256": "51da8fa6ce4ac9b963d4babbd78714e98880b20e74f30a3f45a96334e12830bd",
        "size_mb": 67.3,
    },
    "darwin-arm64": {
        "binary": "opa_darwin_arm64_static",
        "sha256": "fe2a14b6ba7f587caeb62ef93ef62d1e713776a6e470f4e87326468a8ecfbfbd",
        "size_mb": 43.8,
    },
    "linux-x86_64": {
        "binary": "opa_linux_amd64",
        "sha256": "7426bf5504049d7444f9ee9a1d47a64261842f38f5308903ef6b76ba90250b5a",
        "size_mb": 67.1,
    },
    "linux-aarch64": {
        "binary": "opa_linux_arm64_static",
        "sha256": "a81af8cd767f1870e9e23b8ed0ad8f40b24e5c0a64c5768c75d5c292aaa81e54",
        "size_mb": 43.2,
    },
    "win32-x86_64": {
        "binary": "opa_windows_amd64.exe",
        "sha256": "205f87d0fd1e2673c3a6f9caf9d9655290e478a93eeb3ef9f211acdaa214a9ca",
        "size_mb": 98.7,
    },
}

_executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="cupcake-opa")


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

async def ensure_opa_installed() -> Path:
    """Ensure OPA is installed and return its path (async).

    Uses an existing OPA if found, otherwise downloads it.
    """
    loop = asyncio.get_running_loop()
    return await loop.run_in_executor(_executor, ensure_opa_installed_sync)


def ensure_opa_installed_sync() -> Path:
    """Ensure OPA is installed and return its path (blocking)."""
    existing = _find_opa()
    if existing is not None:
        return existing
    return download_opa()


def download_opa(*, force: bool = False) -> Path:
    """Download and verify the OPA binary for the current platform.

    Args:
        force: Re-download even if the binary already exists.

    Returns:
        Path to the OPA executable.
    """
    key = _platform_key()
    if key not in _OPA_BINARIES:
        supported = ", ".join(sorted(_OPA_BINARIES))
        raise RuntimeError(
            f"Unsupported platform: {key}\nSupported platforms: {supported}"
        )

    info = _OPA_BINARIES[key]
    binary_name: str = info["binary"]  # type: ignore[assignment]
    expected_sha256: str = info["sha256"]  # type: ignore[assignment]
    size_mb: float = info["size_mb"]  # type: ignore[assignment]

    cache_dir = _cache_dir()
    local_name = f"opa-{OPA_VERSION}"
    if sys.platform == "win32":
        local_name += ".exe"
    local_path = cache_dir / local_name

    # Check if already downloaded and valid
    if local_path.exists() and not force:
        logger.info("Verifying existing OPA binary at %s ...", local_path)
        if _verify_checksum(local_path, expected_sha256):
            logger.info("Checksum verified.")
            return local_path
        logger.warning("Checksum mismatch — re-downloading.")
        local_path.unlink()

    url = f"{_OPA_BASE_URL}/{binary_name}"
    tmp_path = local_path.with_suffix(".tmp")

    try:
        _download(url, tmp_path, size_mb)

        logger.info("Verifying checksum ...")
        if not _verify_checksum(tmp_path, expected_sha256):
            raise RuntimeError(
                f"SHA-256 verification failed for {binary_name}.\n"
                "This could indicate a corrupted download or security issue."
            )

        tmp_path.rename(local_path)
        _make_executable(local_path)

        logger.info("OPA %s installed at %s", OPA_VERSION, local_path)
        return local_path
    except BaseException:
        if tmp_path.exists():
            tmp_path.unlink(missing_ok=True)
        raise


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------

def _platform_key() -> str:
    """Return a normalised ``platform-arch`` string."""
    plat = sys.platform  # linux, darwin, win32
    machine = platform.machine()  # x86_64, arm64, aarch64, AMD64

    arch_map: dict[str, str] = {
        "x86_64": "x86_64",
        "amd64": "x86_64",
        "arm64": "arm64",
        "aarch64": "aarch64",
    }
    arch = arch_map.get(machine.lower())
    if arch is None:
        raise RuntimeError(f"Unsupported architecture: {machine}")

    return f"{plat}-{arch}"


def _cache_dir() -> Path:
    """Return (and create) the OPA cache directory."""
    if sys.platform == "win32":
        base = Path(os.environ.get("LOCALAPPDATA", Path.home() / "AppData" / "Local"))
    else:
        base = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))

    cache = base / "cupcake" / "bin"
    cache.mkdir(parents=True, exist_ok=True)
    return cache


def _verify_checksum(path: Path, expected: str) -> bool:
    sha = hashlib.sha256()
    with open(path, "rb") as fh:
        while chunk := fh.read(1 << 16):
            sha.update(chunk)
    return sha.hexdigest() == expected


def _download(url: str, dest: Path, size_mb: float) -> None:
    logger.info("Downloading OPA %s (%.1f MB) ...", OPA_VERSION, size_mb)

    req = Request(url, headers={"User-Agent": "cupcake-py"})
    with urlopen(req) as resp, open(dest, "wb") as fh:  # noqa: S310
        total = int(resp.headers.get("Content-Length", 0))
        downloaded = 0
        while chunk := resp.read(1 << 16):
            fh.write(chunk)
            downloaded += len(chunk)
            if total > 0:
                pct = downloaded / total * 100
                print(f"\rProgress: {pct:.1f}%", end="", flush=True)

    print()  # newline after progress
    logger.info("Download complete.")


def _make_executable(path: Path) -> None:
    if sys.platform != "win32":
        path.chmod(path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def _find_opa() -> Path | None:
    """Find an existing OPA binary (cache first, then PATH)."""
    # Check cache
    cache = _cache_dir()
    local_name = f"opa-{OPA_VERSION}"
    if sys.platform == "win32":
        local_name += ".exe"
    cached = cache / local_name
    if cached.exists():
        return cached

    # Check system PATH
    opa_cmd = "opa.exe" if sys.platform == "win32" else "opa"
    system_opa = shutil.which(opa_cmd)
    if system_opa is not None:
        return Path(system_opa)

    return None
