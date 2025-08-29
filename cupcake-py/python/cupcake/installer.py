"""
OPA Binary Installer for Cupcake

This module handles automatic download and verification of the OPA binary
required for policy compilation.

Version: OPA v1.7.1
"""

import hashlib
import os
import platform
import shutil
import stat
import subprocess
import sys
import urllib.request
from pathlib import Path
from typing import Optional, Tuple

# OPA Version Configuration
OPA_VERSION = "v1.7.1"
OPA_BASE_URL = f"https://github.com/open-policy-agent/opa/releases/download/{OPA_VERSION}"

# Platform-specific binary mapping with SHA256 checksums
OPA_BINARIES = {
    ("Darwin", "x86_64"): {
        "binary": "opa_darwin_amd64",
        "sha256": "51da8fa6ce4ac9b963d4babbd78714e98880b20e74f30a3f45a96334e12830bd",
        "size_mb": 67.3
    },
    ("Darwin", "arm64"): {
        "binary": "opa_darwin_arm64_static", 
        "sha256": "fe2a14b6ba7f587caeb62ef93ef62d1e713776a6e470f4e87326468a8ecfbfbd",
        "size_mb": 43.8
    },
    ("Linux", "x86_64"): {
        "binary": "opa_linux_amd64",
        "sha256": "7426bf5504049d7444f9ee9a1d47a64261842f38f5308903ef6b76ba90250b5a",
        "size_mb": 67.1
    },
    ("Linux", "aarch64"): {
        "binary": "opa_linux_arm64_static",
        "sha256": "a81af8cd767f1870e9e23b8ed0ad8f40b24e5c0a64c5768c75d5c292aaa81e54",
        "size_mb": 43.2
    },
    ("Windows", "AMD64"): {
        "binary": "opa_windows_amd64.exe",
        "sha256": "205f87d0fd1e2673c3a6f9caf9d9655290e478a93eeb3ef9f211acdaa214a9ca",
        "size_mb": 98.7
    },
}


def get_cache_dir() -> Path:
    """Get the directory for caching OPA binaries"""
    if sys.platform == "win32":
        cache_base = Path(os.environ.get("LOCALAPPDATA", Path.home() / "AppData" / "Local"))
    else:
        cache_base = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
    
    cache_dir = cache_base / "cupcake" / "bin"
    cache_dir.mkdir(parents=True, exist_ok=True)
    return cache_dir


def get_platform_info() -> Tuple[str, str]:
    """Detect the current platform and architecture"""
    system = platform.system()
    machine = platform.machine()
    
    # Normalize platform names
    if machine in ("x86_64", "AMD64"):
        machine = "x86_64"
    elif machine in ("arm64", "aarch64"):
        machine = "arm64" if system == "Darwin" else "aarch64"
    
    return system, machine


def verify_checksum(file_path: Path, expected_sha256: str) -> bool:
    """Verify the SHA256 checksum of a file"""
    sha256_hash = hashlib.sha256()
    with open(file_path, "rb") as f:
        for chunk in iter(lambda: f.read(4096), b""):
            sha256_hash.update(chunk)
    
    actual = sha256_hash.hexdigest()
    return actual == expected_sha256


def download_with_progress(url: str, dest_path: Path, expected_size_mb: float) -> None:
    """Download a file with progress indication"""
    print(f"Downloading OPA {OPA_VERSION} ({expected_size_mb:.1f} MB)...")
    
    with urllib.request.urlopen(url) as response:
        total_size = int(response.headers.get('Content-Length', 0))
        downloaded = 0
        
        with open(dest_path, 'wb') as f:
            while True:
                chunk = response.read(8192)
                if not chunk:
                    break
                
                f.write(chunk)
                downloaded += len(chunk)
                
                if total_size > 0:
                    percent = (downloaded / total_size) * 100
                    print(f"Progress: {percent:.1f}%", end='\r')
    
    print("\nDownload complete!")


def make_executable(path: Path) -> None:
    """Make a file executable on Unix-like systems"""
    if sys.platform != "win32":
        st = path.stat()
        path.chmod(st.st_mode | stat.S_IEXEC)


def download_opa(force: bool = False) -> Path:
    """
    Download and verify the OPA binary for the current platform
    
    Args:
        force: Force re-download even if binary exists
    
    Returns:
        Path to the OPA executable
    
    Raises:
        RuntimeError: If platform is unsupported or download fails
    """
    # Get platform info
    system, machine = get_platform_info()
    platform_key = (system, machine)
    
    if platform_key not in OPA_BINARIES:
        raise RuntimeError(
            f"Unsupported platform: {system} {machine}\n"
            f"Supported platforms: {list(OPA_BINARIES.keys())}"
        )
    
    binary_info = OPA_BINARIES[platform_key]
    binary_name = binary_info["binary"]
    expected_sha256 = binary_info["sha256"]
    size_mb = binary_info["size_mb"]
    
    # Determine local path
    cache_dir = get_cache_dir()
    local_name = f"opa-{OPA_VERSION}"
    if sys.platform == "win32":
        local_name += ".exe"
    local_path = cache_dir / local_name
    
    # Check if already downloaded and valid
    if local_path.exists() and not force:
        print(f"Verifying existing OPA binary at {local_path}...")
        if verify_checksum(local_path, expected_sha256):
            print("Checksum verified successfully!")
            return local_path
        else:
            print("Checksum verification failed, re-downloading...")
            local_path.unlink()
    
    # Download the binary
    url = f"{OPA_BASE_URL}/{binary_name}"
    temp_path = local_path.with_suffix('.tmp')
    
    try:
        download_with_progress(url, temp_path, size_mb)
        
        # Verify checksum
        print("Verifying checksum...")
        if not verify_checksum(temp_path, expected_sha256):
            raise RuntimeError(
                f"Checksum verification failed for {binary_name}\n"
                f"This could indicate a corrupted download or security issue."
            )
        
        # Move to final location
        temp_path.rename(local_path)
        make_executable(local_path)
        
        print(f"OPA {OPA_VERSION} installed successfully at {local_path}")
        return local_path
        
    except Exception as e:
        # Clean up on failure
        if temp_path.exists():
            temp_path.unlink()
        raise RuntimeError(f"Failed to download OPA: {e}")


def find_opa() -> Optional[Path]:
    """
    Find OPA binary in order of preference:
    1. CUPCAKE_OPA_PATH environment variable
    2. Cached download
    3. System PATH
    
    Returns:
        Path to OPA or None if not found
    """
    # Check environment variable
    env_path = os.environ.get("CUPCAKE_OPA_PATH")
    if env_path:
        path = Path(env_path)
        if path.exists():
            return path
    
    # Check cache
    cache_dir = get_cache_dir()
    local_name = f"opa-{OPA_VERSION}"
    if sys.platform == "win32":
        local_name += ".exe"
    cached_path = cache_dir / local_name
    if cached_path.exists():
        return cached_path
    
    # Check system PATH
    opa_cmd = "opa.exe" if sys.platform == "win32" else "opa"
    opa_path = shutil.which(opa_cmd)
    if opa_path:
        # Verify it's the right version
        try:
            result = subprocess.run(
                [opa_path, "version"],
                capture_output=True,
                text=True,
                timeout=5
            )
            if OPA_VERSION.lstrip('v') in result.stdout:
                return Path(opa_path)
        except:
            pass
    
    return None


def ensure_opa_installed() -> Path:
    """
    Ensure OPA is installed and return its path
    
    This is the main entry point for the installer.
    It will use an existing OPA if found, or download it if needed.
    
    Returns:
        Path to the OPA executable
    
    Raises:
        RuntimeError: If OPA cannot be installed
    """
    # Try to find existing OPA
    opa_path = find_opa()
    if opa_path:
        return opa_path
    
    # Download OPA
    return download_opa()


def get_opa_command() -> str:
    """
    Get the OPA command for subprocess calls
    
    Returns:
        String path to OPA executable
    """
    opa_path = ensure_opa_installed()
    return str(opa_path)


# Module-level convenience
if __name__ == "__main__":
    # Allow running as a script for manual installation
    import argparse
    
    parser = argparse.ArgumentParser(description="Install OPA for Cupcake")
    parser.add_argument("--force", action="store_true", help="Force re-download")
    parser.add_argument("--version", action="store_true", help="Show version")
    
    args = parser.parse_args()
    
    if args.version:
        print(f"OPA version: {OPA_VERSION}")
    else:
        try:
            path = download_opa(force=args.force)
            print(f"OPA is available at: {path}")
        except Exception as e:
            print(f"Error: {e}", file=sys.stderr)
            sys.exit(1)