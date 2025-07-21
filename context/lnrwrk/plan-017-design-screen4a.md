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
