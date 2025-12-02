#!/usr/bin/env python3
"""
Cast Generator - Creates asciicast v3 recordings from castfile definitions.

This script reads castfile YAML definitions and generates .cast files.
Instead of using asciinema recording (which can have output corruption issues),
it creates the cast files directly from the YAML definitions.

Usage:
    ./generate-cast.py <castfile.yaml>           # Generate single cast
    ./generate-cast.py --all                     # Generate all casts
    ./generate-cast.py --list                    # List available castfiles

Requirements:
    - Python 3.8+
    - PyYAML (pip install pyyaml)
"""

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Optional, List, Tuple

try:
    import yaml
except ImportError:
    print("Error: PyYAML not installed. Run: pip install pyyaml")
    sys.exit(1)


# Timing configuration (in seconds)
TIMING = {
    "prompt_pause": 1.0,  # Pause before showing command prompt
    "after_command": 0.5,  # Pause after command before output starts
    "line_delay": 0.15,  # Delay between output lines
    "section_delay": 0.4,  # Delay before section headers (lines with === or starting with caps)
    "between_commands": 1.5,  # Pause between separate commands
    "final_prompt": 2.0,  # Pause before final prompt
}


def parse_duration(duration_str) -> float:
    """Parse duration string like '500ms', '1s', '1.5s' to seconds."""
    if not duration_str:
        return 0

    duration_str = str(duration_str).strip()

    if duration_str.endswith("ms"):
        return float(duration_str[:-2]) / 1000
    elif duration_str.endswith("s"):
        return float(duration_str[:-1])
    else:
        return float(duration_str)


def load_castfile(path: Path) -> dict:
    """Load and validate a castfile YAML."""
    with open(path) as f:
        config = yaml.safe_load(f)

    if not config.get("name"):
        raise ValueError(f"Missing 'name' in {path}")
    if not config.get("steps"):
        raise ValueError(f"Missing 'steps' in {path}")

    config.setdefault("cols", 100)
    config.setdefault("rows", 30)
    config.setdefault("idle_time_limit", 3)
    config.setdefault("setup", [])
    config.setdefault("env", {})
    config.setdefault("title", config["name"])

    return config


def run_command(cmd: str, cwd: Optional[str] = None) -> str:
    """Run a command and capture its output."""
    try:
        result = subprocess.run(
            cmd, shell=True, capture_output=True, text=True, cwd=cwd, timeout=30
        )
        output = result.stdout
        if result.stderr:
            output += result.stderr
        return output
    except subprocess.TimeoutExpired:
        return "[Command timed out]\n"
    except Exception as e:
        return f"[Error: {e}]\n"


def is_section_header(line: str) -> bool:
    """Check if a line looks like a section header."""
    stripped = line.strip()
    if not stripped:
        return False
    # Lines with === or ---
    if "===" in stripped or "---" in stripped:
        return True
    # Lines starting with emoji or check marks
    if stripped and stripped[0] in "✅❌⚠️📜🔒":
        return True
    # Lines that are all caps with colons (like "Commands:" "Options:")
    if stripped.endswith(":") and len(stripped) > 3:
        return True
    return False


def split_output_to_events(
    output: str, base_delay: float = 0.15
) -> List[Tuple[float, str, str]]:
    """
    Split command output into individual line events with appropriate timing.

    Returns list of [delay, "o", content] events.
    """
    events: List[Tuple[float, str, str]] = []
    lines = output.split("\n")

    for i, line in enumerate(lines):
        # Determine delay for this line
        if i == 0:
            delay = TIMING["after_command"]
        elif is_section_header(line):
            delay = TIMING["section_delay"]
        elif line.strip() == "":
            delay = base_delay * 0.5  # Faster for blank lines
        else:
            delay = base_delay

        # Format line for terminal output
        content = line + "\r\n"
        events.append((delay, "o", content))

    return events


def generate_cast(config: dict, output_path: Path, verbose: bool = False):
    """Generate a cast file from config."""

    # Create header
    header = {
        "version": 3,
        "term": {"cols": config["cols"], "rows": config["rows"]},
        "timestamp": int(time.time()),
        "idle_time_limit": config["idle_time_limit"],
        "title": config["title"],
    }

    events: List[List] = []
    is_first_command = True

    # Run setup commands to prepare environment
    setup_dir: Optional[str] = None
    if config.get("setup"):
        import tempfile

        setup_dir = tempfile.mkdtemp()
        for cmd in config["setup"]:
            run_command(cmd, cwd=setup_dir)

    # Process steps
    for step in config["steps"]:
        step_type = step.get("type", "run")

        if step_type == "run":
            command = step.get("command", "")
            wait = parse_duration(step.get("wait", "0"))
            hidden = step.get("hidden", False)

            if not hidden:
                # Add pause before command (longer between commands)
                if is_first_command:
                    prompt_delay = TIMING["prompt_pause"]
                    is_first_command = False
                else:
                    prompt_delay = TIMING["between_commands"]

                # Show the command prompt
                events.append([prompt_delay, "o", f"$ {command}\r\n"])

                # Run the command and capture output
                output = run_command(command, cwd=setup_dir)

                # Split output into individual line events with timing
                if output.strip():
                    line_events = split_output_to_events(output)
                    for evt in line_events:
                        events.append([evt[0], evt[1], evt[2]])
            else:
                # Just run the command silently
                run_command(command, cwd=setup_dir)

            # Add explicit wait if specified
            if wait > 0:
                # Add a no-op event with the wait time
                events.append([wait, "o", ""])

        elif step_type == "wait":
            duration = parse_duration(step.get("duration", "1s"))
            events.append([duration, "o", ""])

        elif step_type == "clear":
            events.append([0.1, "o", "\x1b[2J\x1b[H"])

    # Add final prompt
    events.append([TIMING["final_prompt"], "o", "\r\n$ "])

    # Clean up temp directory
    if setup_dir:
        import shutil

        shutil.rmtree(setup_dir, ignore_errors=True)

    # Filter out empty content events (used just for timing) by merging delays
    filtered_events: List[List] = []
    accumulated_delay = 0.0

    for event in events:
        delay, event_type, content = event
        accumulated_delay += delay

        if content:  # Only add events with actual content
            filtered_events.append([accumulated_delay, event_type, content])
            accumulated_delay = 0.0

    # Write cast file
    with open(output_path, "w") as f:
        f.write(json.dumps(header) + "\n")
        for event in filtered_events:
            f.write(json.dumps(event) + "\n")

    return True


def generate_single(castfile_path: Path, output_dir: Path, verbose: bool = False):
    """Generate a single cast file."""

    print(f"Generating: {castfile_path.stem}")

    config = load_castfile(castfile_path)
    output_path = output_dir / f"{config['name']}.cast"

    if not generate_cast(config, output_path, verbose):
        return False

    size = output_path.stat().st_size
    size_kb = size / 1024
    print(f"  Created: {output_path.name} ({size_kb:.1f} KB)")

    return True


def find_castfiles(base_dir: Path) -> list:
    """Find all castfile YAML files."""
    castfiles = []
    casts_dir = base_dir / "casts"

    for yaml_file in casts_dir.rglob("*.yaml"):
        if yaml_file.name == "schema.yaml":
            continue
        castfiles.append(yaml_file)

    return sorted(castfiles)


def main():
    parser = argparse.ArgumentParser(
        description="Generate asciicast recordings from castfile YAML definitions"
    )
    parser.add_argument(
        "castfile",
        nargs="?",
        help="Path to castfile YAML, or name (e.g., 'help' for cli/help.yaml)",
    )
    parser.add_argument("--all", action="store_true", help="Generate all castfiles")
    parser.add_argument("--list", action="store_true", help="List available castfiles")
    parser.add_argument(
        "--output-dir",
        "-o",
        type=Path,
        help="Output directory for .cast files (default: ../docs/assets/)",
    )
    parser.add_argument(
        "--verbose", "-v", action="store_true", help="Show verbose output"
    )

    args = parser.parse_args()

    script_dir = Path(__file__).parent.resolve()
    base_dir = script_dir

    if args.output_dir:
        output_dir = args.output_dir.resolve()
    else:
        output_dir = base_dir.parent / "docs" / "assets"

    output_dir.mkdir(parents=True, exist_ok=True)

    if args.list:
        print("Available castfiles:")
        print()
        for castfile in find_castfiles(base_dir):
            rel_path = castfile.relative_to(base_dir / "casts")
            name = rel_path.stem
            print(f"  {name:<20} ({rel_path})")
        return

    if args.all:
        print("Generating all cast files...")
        print()

        castfiles = find_castfiles(base_dir)
        if not castfiles:
            print("No castfiles found!")
            return

        success = 0
        failed = 0

        for castfile in castfiles:
            if generate_single(castfile, output_dir, args.verbose):
                success += 1
            else:
                failed += 1

        print()
        print(f"Done: {success} succeeded, {failed} failed")

        print()
        print("Generated files:")
        for cast in sorted(output_dir.glob("*.cast")):
            size_kb = cast.stat().st_size / 1024
            print(f"  {cast.name:<30} {size_kb:>6.1f} KB")

        return

    if not args.castfile:
        parser.print_help()
        return

    castfile_path = None

    if Path(args.castfile).exists():
        castfile_path = Path(args.castfile)
    else:
        name = args.castfile.replace(".yaml", "")

        for subdir in ["cli", "getting-started", ""]:
            candidate = base_dir / "casts" / subdir / f"{name}.yaml"
            if candidate.exists():
                castfile_path = candidate
                break

    if not castfile_path:
        print(f"Error: Castfile not found: {args.castfile}")
        print()
        print("Looked in:")
        print("  - Direct path")
        print("  - casts/cli/<name>.yaml")
        print("  - casts/getting-started/<name>.yaml")
        print("  - casts/<name>.yaml")
        sys.exit(1)

    generate_single(castfile_path, output_dir, args.verbose)


if __name__ == "__main__":
    main()
