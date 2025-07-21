Looking at Screen 4 more carefully:

## Screen 4: Rule Review & Edit

This screen is actually simpler than I was making it:

### Main Features:

1. **List of extracted rules** grouped by source file
2. **Checkboxes** to select/deselect rules (☑/☐)
3. **Search bar** at the top to filter rules
4. **Severity badges** (🔴 Critical, 🟡 Warning, 🔵 Info)
5. **Tree structure** with collapsible sections (▼/▶)

### What's happening with search:

```
│ ┌───────────────────────────────────────────────────────────────────┐  │
│ │ Search: test█                                              3 matches│  │
│ └───────────────────────────────────────────────────────────────────┘  │
```

When user types "test", it:

- Filters the list to show only rules containing "test"
- Shows "3 matches" count
- The `»test«` notation in the list appears to indicate the matching text

### The "dropdown" confusion:

I was overthinking because of **Screen 4a (Edit Modal)**. In the edit modal, there are dropdown-style fields:

```
│ Severity:    [🔴 Critical    ▼]       │
│ Category:    [Testing        ▼]       │
│ When:        [pre-commit     ▼]       │
```

But these aren't real dropdowns - they're just fields that change value when you press up/down arrows.

### Simplified Implementation for Screen 4:

```rust
struct ReviewState {
    rules: Vec<ExtractedRule>,
    selected: HashSet<usize>,
    search_input: Input,
    search_active: bool,
    list_state: ListState,
    expanded_sections: HashSet<String>, // Which sections are expanded
}

// Rendering simplified
fn render_rule_list(&self, f: &mut Frame, area: Rect) {
    let search_query = self.search_input.value().to_lowercase();

    let mut items = vec![];
    for (source, rules) in &self.grouped_rules {
        // Section header
        let icon = if self.expanded_sections.contains(source) { "▼" } else { "▶" };
        items.push(ListItem::new(format!("{} {} ({} rules)", icon, source, rules.len())));

        // Rules (if expanded)
        if self.expanded_sections.contains(source) {
            for rule in rules {
                if rule.description.to_lowercase().contains(&search_query) {
                    let checkbox = if self.selected.contains(&rule.id) { "☑" } else { "☐" };
                    let severity_icon = match rule.severity {
                        Severity::Critical => "🔴",
                        Severity::Warning => "🟡",
                        Severity::Info => "🔵",
                    };
                    items.push(ListItem::new(format!(
                        "  {} {} {} {}",
                        checkbox,
                        rule.description,
                        severity_icon,
                        rule.severity
                    )));
                }
            }
        }
    }

    let list = List::new(items)
        .block(Block::bordered().title("Review Extracted Rules"));
    f.render_stateful_widget(list, area, &mut self.list_state);
}
```

### No complex highlighting needed!

The `»test«` notation in the wireframe is just visual indication in the design. We don't need to actually highlight text within the rules. We can either:

1. Just filter the list (simplest)
2. Or show the whole rule with the match count

So Screen 4 is actually quite straightforward - it's just a filterable checklist with collapsible sections!
