# Testing the TUI

To visually test the TUI init wizard, run this in your terminal:

```bash
cargo run --bin test_tui
```

## What you'll see:

1. A split-pane interface with:
   - Left pane: File discovery and selection list
   - Right pane: Preview area (currently placeholder)

2. The discovery will show mock files being found:
   - CLAUDE.md [Claude]
   - .cursor/rules [Cursor]
   - .windsurf/rules/ [Windsurf] with children files
   - .kiro/steering/ [Kiro] with children files
   - copilot-instructions [Copilot]
   - .aider.conf.yml [Aider]

3. A progress bar showing scanning progress

4. Keyboard controls:
   - ↑↓ - Navigate files
   - Space - Toggle selection
   - Tab - Switch between panes
   - Enter - Continue (when files are selected)
   - q - Quit

## Current state:
- Discovery screen is fully implemented
- Mock data is used for testing
- Preview pane shows placeholder text
- Continuing will transition to extraction screen (not yet implemented)

The TUI must be run in a real terminal, not through VS Code's output panel.