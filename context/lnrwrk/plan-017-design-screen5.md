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
