# Cupcake Documentation Assets

This directory contains castfile definitions for generating terminal recordings using asciinema. Generated `.cast` files are output to `docs/docs/assets/` for the Zensical doc site.

## Prerequisites

Install [asciinema](https://docs.asciinema.org/) and PyYAML:

```bash
# macOS
brew install asciinema
pip install pyyaml

# Linux
pip install asciinema pyyaml

# From source (Rust)
cargo install asciinema
pip install pyyaml
```

## Directory Structure

```
docs/
├── assets/                    # Castfile source files (this directory)
│   ├── casts/
│   │   ├── cli/               # CLI command demos
│   │   │   ├── help.yaml
│   │   │   ├── init.yaml
│   │   │   ├── inspect.yaml
│   │   │   ├── verify.yaml
│   │   │   └── trust.yaml
│   │   └── schema.yaml        # Castfile format documentation
│   ├── generate-cast.py       # Cast generator script
│   └── README.md
├── docs/
│   ├── assets/                # Generated .cast files output here
│   │   ├── cupcake-help.cast
│   │   ├── cupcake-init.cast
│   │   └── ...
│   ├── javascripts/           # asciinema-player JS
│   └── stylesheets/           # asciinema-player CSS
└── zensical.toml
```

## Usage

Run these commands from the **repository root**:

### Generate All Assets

```bash
just casts
```

This builds cupcake and generates all `.cast` files from castfile definitions.

### Generate a Specific Asset

```bash
just cast help        # Generate from casts/cli/help.yaml
just cast init        # Generate from casts/cli/init.yaml
just cast inspect     # etc.
```

### List Available Castfiles

```bash
just list-casts
```

## Castfile Format

Castfiles are YAML definitions that describe terminal recording sessions. See `casts/schema.yaml` for the full specification.

### Basic Structure

```yaml
# Required fields
name: my-demo           # Output filename (produces my-demo.cast)
steps:                  # Recording steps
  - type: run
    command: "echo hello"

# Optional fields
title: "My Demo"        # Recording title
description: "..."      # Human-readable description
cols: 100               # Terminal columns (default: 100)
rows: 30                # Terminal rows (default: 30)
idle_time_limit: 2      # Max idle time in seconds (default: 2)
setup:                  # Commands to run before recording (hidden)
  - "cd /tmp"
env:                    # Environment variables
  MY_VAR: "value"
```

### Step Types

| Type    | Description                  | Example                                    |
| ------- | ---------------------------- | ------------------------------------------ |
| `run`   | Execute command, show output | `{type: run, command: "ls -la"}`           |
| `wait`  | Pause recording              | `{type: wait, duration: "1s"}`             |
| `clear` | Clear terminal screen        | `{type: clear}`                            |

### Step Options

```yaml
steps:
  - type: run
    command: "cupcake --help"
    wait: "2s"           # Wait after command (optional)
    hidden: false        # Hide from output (optional)
```

### Duration Format

Durations can be specified as:

- `500ms` - milliseconds
- `1s` - seconds
- `1.5s` - fractional seconds

## Example Castfile

```yaml
name: cupcake-init
title: "Cupcake Init Command"
description: "Initialize cupcake in a project directory"
cols: 100
rows: 30
idle_time_limit: 2

setup:
  - "cd /tmp && rm -rf demo && mkdir demo && cd demo"

steps:
  - type: run
    command: "ls -la"
    wait: "1s"

  - type: run
    command: "cupcake init --harness claude"
    wait: "2s"

  - type: run
    command: "tree -a .cupcake"
    wait: "2s"
```

## Using in Documentation

In markdown files, embed the player with:

```html
<div class="cast-player" data-cast="../assets/cupcake-help.cast"></div>
```

### Optional Attributes

| Attribute       | Description                          | Default   |
| --------------- | ------------------------------------ | --------- |
| `data-cast`     | Path to .cast file (required)        | -         |
| `data-autoplay` | Auto-play on load                    | `false`   |
| `data-loop`     | Loop playback                        | `false`   |
| `data-speed`    | Playback speed multiplier            | `1`       |
| `data-poster`   | Poster frame (e.g., `npt:0:30`)      | -         |
| `data-fit`      | Fit mode: width, height, both, none  | `width`   |

## Generated Assets

After running `just casts`, you'll have `.cast` files in `docs/docs/assets/`:

| File                  | Description                               |
| --------------------- | ----------------------------------------- |
| `cupcake-help.cast`   | Main CLI help output                      |
| `cupcake-init.cast`   | Project initialization                    |
| `cupcake-inspect.cast`| Policy inspection (detailed + table view) |
| `cupcake-verify.cast` | Configuration verification                |
| `cupcake-trust.cast`  | Trust management workflow                 |

## Troubleshooting

### asciinema not found

Install asciinema using your package manager or pip:

```bash
brew install asciinema  # macOS
pip install asciinema   # cross-platform
```

### PyYAML not found

Install the Python YAML library:

```bash
pip install pyyaml
```

### Cast file is empty or corrupted

1. Check that the commands in your castfile work when run manually
2. Ensure cupcake is built: `just build-cli`
3. Try running with verbose output: `python3 docs/assets/generate-cast.py help -v`

### Player not loading

1. Verify the `.cast` file exists and is valid JSON
2. Check browser console for JavaScript errors
3. Ensure the path in `data-cast` is correct relative to the markdown file
