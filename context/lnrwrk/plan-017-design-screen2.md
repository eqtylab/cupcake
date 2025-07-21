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
