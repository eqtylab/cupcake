#!/usr/bin/env python3
"""
Guidebook Verification Script

Verifies guidebook.yml files in the development environment don't
reference deprecated environment variables.

This is a DEVELOPMENT VERIFICATION tool only - not for user migration
(pre-release, no users exist yet).

Usage:
    python scripts/verify_guidebooks.py [path/to/guidebook.yml]
    python scripts/verify_guidebooks.py  # Scans common locations
"""

import sys
import yaml
from pathlib import Path
from typing import List, Tuple


# Environment variables that have been deprecated
DEPRECATED_ENV_VARS = {
    "CUPCAKE_TRACE",
    "RUST_LOG",
    "CUPCAKE_GLOBAL_CONFIG",
    "CUPCAKE_WASM_MAX_MEMORY",
    "CUPCAKE_DEBUG_FILES",
    "CUPCAKE_DEBUG_ROUTING",
    "CUPCAKE_OPA_PATH",
}


def scan_yaml_for_env_vars(config: dict, path: Path) -> List[Tuple[str, str]]:
    """Scan YAML config for deprecated env var references"""
    findings = []

    # Check signals
    if 'signals' in config:
        for signal_name, signal_config in config['signals'].items():
            if isinstance(signal_config, dict) and 'command' in signal_config:
                command = str(signal_config['command'])
                for env_var in DEPRECATED_ENV_VARS:
                    if f"${env_var}" in command or f"${{{env_var}}}" in command:
                        findings.append((
                            f"Signal '{signal_name}' in {path}",
                            f"References deprecated ${env_var}"
                        ))

    # Check actions
    if 'actions' in config:
        for action_name, action_config in config['actions'].items():
            if isinstance(action_config, dict) and 'command' in action_config:
                command = str(action_config['command'])
                for env_var in DEPRECATED_ENV_VARS:
                    if f"${env_var}" in command or f"${{{env_var}}}" in command:
                        findings.append((
                            f"Action '{action_name}' in {path}",
                            f"References deprecated ${env_var}"
                        ))

    return findings


def verify_guidebook(path: Path) -> bool:
    """Verify a single guidebook file"""
    print(f"Checking: {path}")

    try:
        with open(path, 'r') as f:
            config = yaml.safe_load(f) or {}
    except Exception as e:
        print(f"  ⚠️  Error reading file: {e}")
        return False

    findings = scan_yaml_for_env_vars(config, path)

    if findings:
        print(f"  ✗ Found {len(findings)} deprecated env var references:")
        for location, issue in findings:
            print(f"    • {location}")
            print(f"      {issue}")
        return False
    else:
        print(f"  ✓ No deprecated env vars found")
        return True


def find_guidebooks() -> List[Path]:
    """Find all guidebook.yml files in common locations"""
    search_paths = [
        Path(".cupcake/guidebook.yml"),
        Path("fixtures/init/base-config.yml"),
        Path("fixtures/builtins"),
        Path("fixtures/global_builtins"),
        Path("eval"),
    ]

    guidebooks = []

    for search_path in search_paths:
        if search_path.is_file():
            guidebooks.append(search_path)
        elif search_path.is_dir():
            guidebooks.extend(search_path.rglob("*.yml"))
            guidebooks.extend(search_path.rglob("guidebook.yml"))

    return list(set(guidebooks))  # Remove duplicates


def main():
    """Main entry point"""
    print("=" * 70)
    print("Guidebook Verification (Development)")
    print("=" * 70)
    print()

    if len(sys.argv) > 1:
        # Check specific file
        guidebooks = [Path(sys.argv[1])]
    else:
        # Scan common locations
        print("Scanning for guidebook.yml files...")
        guidebooks = find_guidebooks()
        print(f"Found {len(guidebooks)} files to check")
        print()

    all_clean = True

    for guidebook in guidebooks:
        if not guidebook.exists():
            print(f"File not found: {guidebook}")
            all_clean = False
            continue

        if not verify_guidebook(guidebook):
            all_clean = False

        print()

    print("=" * 70)
    if all_clean:
        print("✓ All guidebooks verified - no deprecated env vars")
        print()
        print("Development environment is ready for refactor.")
        sys.exit(0)
    else:
        print("✗ Some guidebooks reference deprecated env vars")
        print()
        print("Fix these before completing Phase 1 refactor.")
        sys.exit(1)


if __name__ == "__main__":
    main()
