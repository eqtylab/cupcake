# Cupcake Init - Final UX Flow with Ratatui Implementation

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

## Screen 2: Custom Extraction Instructions (Optional)

**UX Description:** Modal overlay on previous screen. Text input area with word wrap. Dismissible with 's' to skip. Auto-focuses text area.

**Ratatui Implementation:**

- `Clear` widget to create modal effect
- `Block` with `BorderType::Rounded` for modal appearance
- `Paragraph` widget with custom `TextArea` state management
- Centered using `Layout` with `Constraint::Length` and margins

```
┌─ Select Rule Sources ─────────────────────┬─ Preview ────────────────────┐
│ ☑ CLAUDE.md                    [Claude]   │ CLAUDE.md                    │
│ ☑ .cursor/rules        ┌─ Custom Instructions (Optional) ─────────────┐ │
│ ☐ .windsurf/rules/     │                                               │ │
│   ├─ formatting.md     │ Add context for rule extraction:              │ │
│   ├─ security.md       │ ┌───────────────────────────────────────────┐ │ │
│   └─ performance.md    │ │ Focus on security and compliance rules.   │ │ │
│ ☑ .kiro/steering/      │ │ Our team uses "MUST" for critical rules  │ │ │
│   ├─ agent-policy.yml  │ │ and "SHOULD" for recommendations.█        │ │ │
│   └─ constraints.yml   │ └───────────────────────────────────────────┘ │ │
│ ☐ copilot-instructions │                                               │ │
│ ☑ .aider.conf.yml      │ 💡 Helps AI understand your conventions       │ │
│                        │                                               │ │
│ Selected: 4 sources    │ [Enter] Apply  [s] Skip  [Esc] Cancel         │ │
└────────────────────────└───────────────────────────────────────────────┘ │
 [↑↓] Navigate  [Space] Toggle  [Tab] Switch Pane  [Enter] Continue  [q] Quit
```

## Screen 3: Parallel Rule Extraction

**UX Description:** Real-time progress for each file processing in parallel. Shows timing, progress bars, and extracted rule counts. Failed extractions show retry option.

**Ratatui Implementation:**

- `Table` widget with dynamic row updates
- Custom `Sparkline` or mini `Gauge` widgets per row
- `Span` with colors for status indicators (green ✓, yellow ⟳, red ✗)
- Bottom `Gauge` for overall progress
- Async updates via channels from parallel extraction tasks

```
┌─ Extracting Rules ───────────────────────────────────────────────────────┐
│                                                                          │
│ Processing 7 files in parallel...                           GPT-4 Turbo │
│                                                                          │
│ File                          Progress    Status         Time     Rules  │
│ ──────────────────────────────────────────────────────────────────────  │
│ CLAUDE.md                    ████████████ ✓ Complete    142ms    12     │
│ .cursor/rules                ████████░░░░ ⟳ Extract 72% 189ms    8/11   │
│ .windsurf/rules/formatting   ████████████ ✓ Complete    97ms     8      │
│ .windsurf/rules/security     ██░░░░░░░░░░ ⟳ Extract 15% 54ms     --     │
│ .windsurf/rules/performance  ░░░░░░░░░░░░ ⏳ Queued      --ms     --     │
│ .kiro/steering/agent-policy  ████████████ ✓ Complete    203ms    15     │
│ .aider.conf.yml              ███████████░ ⟳ Extract 89% 167ms    6/7    │
│                                                                          │
│ ──────────────────────────────────────────────────────────────────────  │
│ Overall: ████████████░░░░░░  62%        35 of 56 rules extracted        │
│                                                                          │
│ 💡 Tip: Extraction uses your custom instructions                         │
└──────────────────────────────────────────────────────────────────────────┘
 [r] Retry failed  [p] Pause  [Esc] Cancel
```

## Screen 4: Rule Review & Edit

**UX Description:** Hierarchical rule display grouped by source. Collapsible sections. Inline search with highlighting. Bulk operations. Color-coded severity badges.

**Ratatui Implementation:**

- Custom `StatefulList` with tree structure
- `Paragraph` widgets for rule text with `Style` for severity colors
- Search using `Input` mode with live filtering
- Modal `Popup` for editing (reuses code from Screen 2)
- `Scrollbar` widget for long lists

```
┌─ Review Extracted Rules ─────────────────────────────────────────────────┐
│ 56 rules found • 52 selected              [/] Search  [a] All  [n] None │
│ ┌───────────────────────────────────────────────────────────────────┐  │
│ │ Search: test█                                              3 matches│  │
│ └───────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│ ▼ CLAUDE.md (12 rules) ────────────────────────────────── 11 selected   │
│   ☑ Always run »test« before committing                  🔴 Critical   │
│   ☑ Use TypeScript strict mode                           🟡 Warning    │
│   ☑ Document all public APIs                             🔵 Info       │
│                                                                          │
│ ▶ .cursor/rules (8 rules) ─────────────────────────────── 8 selected    │
│                                                                          │
│ ▼ .kiro/steering (15 rules) ───────────────────────────── 14 selected   │
│   ☑ Require PR approval before merging                   🔴 Critical   │
│   ☑ All »test«s must pass in CI                          🔴 Critical   │
│   ☐ Format code with prettier on save                    🔵 Info       │
│   ☑ No force-push to protected branches                  🔴 Critical   │
│                                                         Scroll ▼ 40%   │
└──────────────────────────────────────────────────────────────────────────┘
 [↑↓] Navigate  [Space] Toggle  [e] Edit  [Enter] Continue  [q] Back
```

## Screen 4a: Rule Edit Modal

**UX Description:** Overlay modal with form fields. Tab navigation between fields. Dropdown menus for enums. Real-time validation.

**Ratatui Implementation:**

- Modal using `Clear` + centered `Block`
- Multiple `Paragraph` widgets for form fields
- Custom dropdown using `List` widget in a small window
- Tab-based focus management with highlighted borders

```
┌─ Review Extracted Rules ─────────────────────────────────────────────────┐
│ 56 rules found • 52 selected   ┌─ Edit Rule ──────────────────────────┐ │
│                                │                                       │ │
│ ▼ CLAUDE.md (12 rules) ────────│ Description:                          │ │
│   ☑ Always run tests before co │ ┌───────────────────────────────────┐ │ │
│   ☑ Use TypeScript strict mode │ │ Always run tests before           │ │ │
│   ☑ Document all public APIs   │ │ committing code changes█          │ │ │
│                                │ └───────────────────────────────────┘ │ │
│ ▶ .cursor/rules (8 rules) ─────│                                       │ │
│                                │ Severity:    [🔴 Critical    ▼]       │ │
│ ▼ .kiro/steering (15 rules) ───│ Category:    [Testing        ▼]       │ │
│   ☑ Require PR approval before │ When:        [pre-commit     ▼]       │ │
│   ☑ All tests must pass in CI  │                                       │ │
│   ☐ Format code with prettier  │ ☑ Block action on violation           │ │
│   ☑ No force-push to protected │ ☐ Warn only                           │ │
│                                │                                       │ │
└────────────────────────────────│ [Ctrl+Enter] Save  [Esc] Cancel      │ │
                                 └───────────────────────────────────────┘ │
```

## Screen 5: Compilation & Sync

**UX Description:** Multi-phase progress with detailed status. Collapsible log viewer. Real-time updates. Clear phase indicators.

**Ratatui Implementation:**

- Multiple `Gauge` widgets for different phases
- `List` widget for log output with auto-scroll
- `Block` titles update to show current phase
- Status icons using Unicode symbols with colors

```
┌─ Finalizing Configuration ───────────────────────────────────────────────┐
│                                                                          │
│ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░  78%  Phase 2 of 3    │
│                                                                          │
│ ✓ Phase 1: Policy Compilation                              348ms        │
│   └─ Generated 3 policy files (18 critical, 23 warning, 11 info)       │
│                                                                          │
│ ⟳ Phase 2: Claude Code Hook Installation                               │
│   ├─ ✓ Created .claude-code/hooks/ directory                           │
│   ├─ ✓ Installed pre-commit hook                                       │
│   ├─ ⟳ Installing file-change watchers...                              │
│   └─ ⏳ Pending: post-edit validator                                    │
│                                                                          │
│ ⏳ Phase 3: Validation & Testing                                        │
│                                                                          │
│ ┌─ Installation Log ─────────────────────────────────────────[▼ Hide]─┐ │
│ │ [12:34:01] Writing blocking-policies.yml...                        │ │
│ │ [12:34:01] Optimizing regex patterns...                            │ │
│ │ [12:34:02] Hook registered: pre-commit → cupcake.check             │ │
│ └────────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────┘
 [l] Toggle logs  [v] Verbose  [Esc] Cancel
```

## Screen 6: Success Summary

**UX Description:** Clean success screen with actionable next steps. Quick action buttons. Copy-able commands. Performance metrics.

**Ratatui Implementation:**

- `Paragraph` with styled text (bold, colors)
- `Table` for summary statistics
- Highlighted command blocks using `Block` with different background
- Bottom bar with action shortcuts

```
┌─ ✅ Cupcake Successfully Initialized ────────────────────────────────────┐
│                                                                          │
│  Your AI coding agent now has deterministic guardrails!                 │
│                                                                          │
│  ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓  │
│  ┃ Configuration Summary                                            ┃  │
│  ┣━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫  │
│  ┃ Total Rules          ┃ 52 (from 4 sources)                     ┃  │
│  ┃ Critical (blocking)  ┃ 18 rules - will halt operations        ┃  │
│  ┃ Warning (advisory)   ┃ 23 rules - will show warnings          ┃  │
│  ┃ Info (logging only)  ┃ 11 rules - tracked for metrics         ┃  │
│  ┃ Performance Impact   ┃ ~8ms average per file operation        ┃  │
│  ┃ Config Location      ┃ .cupcake/config.yml                    ┃  │
│  ┗━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛  │
│                                                                          │
│  Try these commands:                                                     │
│  ┌────────────────────────────────────────────────────────────────┐    │
│  │ $ cupcake test              # Validate your configuration      │    │
│  │ $ cupcake status            # View active policies             │    │
│  │ $ echo "TODO: fix" > test.py # See Cupcake in action!         │    │
│  └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
 [Enter] Exit  [t] Test now  [s] Show status  [d] Documentation

```

## Ratatui Architecture Summary

**Key Components:**

- **State Machine**: Enum-based state management for screen transitions
- **Event Loop**: Crossterm events + custom app events via channels
- **Async Integration**: Tokio for file discovery, LLM calls, and compilation
- **Widget Composition**: Reusable components for modals, progress bars, file trees
- **Styling System**: Consistent color scheme using ratatui's `Style` builder

**Performance Optimizations:**

- Virtualized lists for large rule sets (only render visible items)
- Debounced preview updates (50ms delay on cursor movement)
- Background thread for file I/O, results streamed to UI
- Minimal redraws using ratatui's diffing algorithm

This design leverages ratatui's strengths while providing a modern, responsive TUI that feels as smooth as a GUI application.
