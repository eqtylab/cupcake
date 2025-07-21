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
