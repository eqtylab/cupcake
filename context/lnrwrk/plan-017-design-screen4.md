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
