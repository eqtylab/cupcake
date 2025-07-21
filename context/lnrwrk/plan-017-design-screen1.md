## Screen 1: Combined Discovery + Selection

**UX Description:** Dynamic file discovery with immediate interaction. Files appear as found, already selectable. Background scanning continues while users interact. Split pane with live preview updates on cursor movement.

**Ratatui Implementation:**

- `List` widget with custom `ListItem` rendering for checkboxes and badges
- `Gauge` widget for scan progress
- `Block` widgets with borders for panes
- `tokio::spawn` for async file discovery, `mpsc::channel` for UI updates
- `Paragraph` widget for preview pane with `Wrap` enabled

```
┌─ Select Rule Sources ─────────────────────┬─ Preview ────────────────────┐
│ Scanning repository... ████████░░ 78%     │ CLAUDE.md                    │
│                                           │ ──────────                   │
│ ☑ CLAUDE.md                    [Claude]   │                              │
│ ☑ .cursor/rules                [Cursor]   │ # Claude Development Rules   │
│ ☐ .windsurf/rules/           [Windsurf]   │                              │
│   ├─ formatting.md                        │ ## Testing Standards         │
│   ├─ security.md                          │ - Always write tests first   │
│   └─ performance.md                       │ - Minimum 80% coverage       │
│ ☑ .kiro/steering/               [Kiro]    │ - Use descriptive test names │
│   ├─ agent-policy.yml                     │                              │
│   └─ constraints.yml                      │ ## Code Style               │
│ ☐ copilot-instructions       [Copilot]    │ - Use 2-space indentation   │
│ ☑ .aider.conf.yml              [Aider]    │ - Prefer const over let     │
│ ⟳ Searching: .augment*, GEMINI.md...      │ - Max line length: 100      │
│                                           │                              │
│ Selected: 4 sources (7 files)             │ ## Security                  │
└───────────────────────────────────────────┴──────────────────────────────┘
 [↑↓] Navigate  [Space] Toggle  [Tab] Switch Pane  [Enter] Continue  [q] Quit
```
