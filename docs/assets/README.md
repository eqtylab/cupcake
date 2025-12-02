# Cupcake Documentation Assets

This directory contains VHS tape files for generating reproducible GIFs for documentation. Generated GIFs are output directly to `docs/docs/assets/` for the zensical doc site.

## Prerequisites

Install [VHS](https://github.com/charmbracelet/vhs) (includes dependencies):

```bash
# macOS
brew install vhs

# Windows
scoop install vhs

# From source
go install github.com/charmbracelet/vhs@latest
```

VHS requires `ttyd` and `ffmpeg`. The brew/scoop installers handle these automatically.

## Directory Structure

```
docs/
├── assets/                    # VHS tape source files (this directory)
│   ├── tapes/
│   │   ├── common/
│   │   │   └── settings.tape  # Shared terminal settings (theme, font, size)
│   │   ├── cli/               # CLI command demos
│   │   │   ├── help.tape
│   │   │   ├── init.tape
│   │   │   ├── inspect.tape
│   │   │   ├── verify.tape
│   │   │   └── trust.tape
│   │   └── getting-started/   # Getting started demos (future)
│   └── README.md
├── docs/
│   └── assets/                # Generated GIFs output here
│       ├── cupcake-help.gif
│       ├── cupcake-init.gif
│       └── ...
└── zensical.toml
```

## Usage

Run these commands from the **repository root**:

### Generate All Assets

```bash
just assets
```

This builds cupcake and generates all GIFs from tape files into `docs/docs/assets/`.

### Generate a Specific Asset

```bash
just asset help        # Generate from cli/help.tape
just asset init        # Generate from cli/init.tape
just asset inspect     # etc.
```

### List Available Tapes

```bash
just list-tapes
```

### Preview a Tape (Dry Run)

```bash
just preview-tape help
```

## Writing Tape Files

### Basic Structure

All paths in tape files are relative to the `docs/assets/` directory (VHS runs from there).

```tape
# Source common settings
Source tapes/common/settings.tape

# Output to docs/docs/assets/ for the doc site
Output ../docs/assets/my-demo.gif

# Hide setup commands
Hide
Type "cd /tmp && mkdir demo && cd demo"
Enter
Sleep 300ms
Type "clear"
Enter
Show

# The actual demo - pipe to less for scrollable output
Type "cupcake --help | less"
Sleep 500ms
Enter
Sleep 2s

# Scroll through output with j/k keys
Type "jjjjjjjjjj"
Sleep 800ms

# Return to top
Type "gg"
Sleep 1s

# Exit less
Type "q"
```

### Scrolling Long Output

For commands with long output, pipe to `less` and use vim-style navigation:

| Key     | Action               |
| ------- | -------------------- |
| `j`     | Scroll down one line |
| `k`     | Scroll up one line   |
| `Space` | Page down            |
| `b`     | Page up              |
| `gg`    | Go to top            |
| `G`     | Go to bottom         |
| `q`     | Quit less            |

### Key Commands

| Command            | Description                                  |
| ------------------ | -------------------------------------------- |
| `Source <file>`    | Include another tape file (use for settings) |
| `Output <path>`    | Set output file (.gif, .mp4, .webm)          |
| `Type "<text>"`    | Type text into the terminal                  |
| `Enter`            | Press Enter key                              |
| `Sleep <duration>` | Wait (e.g., `500ms`, `2s`)                   |
| `Hide` / `Show`    | Hide/show commands from recording            |
| `Ctrl+<key>`       | Press Ctrl+key combo                         |

### Settings (in common/settings.tape)

| Setting                      | Description                              |
| ---------------------------- | ---------------------------------------- |
| `Set Width <px>`             | Terminal width                           |
| `Set Height <px>`            | Terminal height                          |
| `Set FontSize <px>`          | Font size                                |
| `Set FontFamily "<font>"`    | Font family                              |
| `Set Theme "<name>"`         | Color theme (see `vhs themes`)           |
| `Set TypingSpeed <duration>` | Delay between keystrokes                 |
| `Set Padding <px>`           | Terminal padding                         |
| `Set WindowBar <style>`      | Window bar style (Colorful, Rings, etc.) |

## Tips

1. **Use `less` for long output**: Pipe commands to `less` and use j/k to scroll
2. **Use Hide/Show**: Setup and cleanup should be hidden from recording
3. **Add sleep after commands**: Let output render before continuing
4. **Test with dry-run first**: `just preview-tape <name>` validates syntax

## Customizing Theme

The current theme is `Catppuccin Mocha`. To change it, edit `tapes/common/settings.tape`.

List available themes:

```bash
vhs themes
```

## Generated Assets

After running `just assets`, you'll have GIFs in `docs/docs/assets/`:

| GIF                   | Description                               |
| --------------------- | ----------------------------------------- |
| `cupcake-help.gif`    | Main CLI help output                      |
| `cupcake-init.gif`    | Project initialization                    |
| `cupcake-inspect.gif` | Policy inspection (detailed + table view) |
| `cupcake-verify.gif`  | Configuration verification                |
| `cupcake-trust.gif`   | Trust management workflow                 |
