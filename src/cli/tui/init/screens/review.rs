//! Rule review and editing screen

use std::collections::HashMap;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use crate::cli::tui::init::state::{ReviewState, Severity, ExtractedRule, RuleEditForm, FormField};

pub fn render(frame: &mut Frame, state: &ReviewState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),      // Header
            Constraint::Length(3),      // Search bar
            Constraint::Min(10),        // Rule list
            Constraint::Length(1),      // Help
        ])
        .split(frame.area());
    
    render_header(frame, chunks[0], state);
    render_search_bar(frame, chunks[1], state);
    render_rule_list(frame, chunks[2], state);
    render_help(frame, chunks[3]);
    
    // Render edit modal if active
    if let Some(rule_idx) = state.editing_rule {
        if let Some(rule) = state.rules.get(rule_idx) {
            render_edit_modal(frame, frame.area(), rule, &state.edit_form);
        }
    }
}

fn render_header(frame: &mut Frame, area: Rect, state: &ReviewState) {
    let total_rules = state.rules.len();
    let selected_rules = state.selected.len();
    
    let header = Line::from(vec![
        Span::raw(format!(" {} rules found • {} selected", total_rules, selected_rules)),
    ]);
    
    let paragraph = Paragraph::new(header)
        .block(Block::default()
            .title(format!(" Choose which rules to enforce ({} of {} selected) ", selected_rules, total_rules))
            .borders(Borders::ALL));
    
    frame.render_widget(paragraph, area);
}

fn render_search_bar(frame: &mut Frame, area: Rect, state: &ReviewState) {
    if state.search_active {
        let input = Paragraph::new(Line::from(vec![
            Span::raw("Search: "),
            Span::raw(state.search_input.value()),
            Span::styled("█", Style::default().fg(Color::White)),
            Span::raw(format!("                              {} matches", 
                state.filtered_indices.len())),
        ]))
        .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(input, area);
    }
}

fn render_rule_list(frame: &mut Frame, area: Rect, state: &ReviewState) {
    // Group rules by source file
    let mut grouped_rules: HashMap<String, Vec<(usize, &ExtractedRule)>> = HashMap::new();
    
    let rules_to_show: Vec<(usize, &ExtractedRule)> = if state.search_active && !state.search_input.value().is_empty() {
        state.filtered_indices.iter()
            .filter_map(|&idx| state.rules.get(idx).map(|r| (idx, r)))
            .collect()
    } else {
        state.rules.iter().enumerate().map(|(idx, r)| (idx, r)).collect()
    };
    
    for (idx, rule) in &rules_to_show {
        let source = rule.source_file.to_string_lossy().to_string();
        grouped_rules.entry(source).or_insert_with(Vec::new).push((*idx, *rule));
    }
    
    let mut items = Vec::new();
    let mut current_line = 0;
    let mut line_to_index = Vec::new();
    
    // Build the list items
    for (source, rules) in grouped_rules.iter() {
        let selected_in_group = rules.iter().filter(|(idx, _)| state.selected.contains(idx)).count();
        let is_expanded = state.expanded_sections.contains(source);
        
        // Section header
        let expand_icon = if is_expanded { "▼" } else { "▶" };
        let section_header = Line::from(vec![
            Span::raw(format!("{} {} ({} rules) ", expand_icon, source, rules.len())),
            Span::raw("─".repeat(20)),
            Span::raw(format!(" {} selected", selected_in_group)),
        ]);
        
        items.push(ListItem::new(section_header).style(Style::default().add_modifier(Modifier::BOLD)));
        line_to_index.push(None); // Section headers don't have an index
        current_line += 1;
        
        // Show rules if expanded
        if is_expanded {
            for (idx, rule) in rules {
                if current_line > state.selected_index.saturating_sub(5) && 
                   current_line < state.selected_index + area.height as usize {
                    let is_selected = state.selected.contains(idx);
                    let is_focused = state.selected_index == current_line;
                    
                    let checkbox = if is_selected { "☑" } else { "☐" };
                    let severity_badge = match rule.severity {
                        Severity::Critical => Span::styled("🔴 Critical", Style::default().fg(Color::Red)),
                        Severity::Warning => Span::styled("🟡 Warning", Style::default().fg(Color::Yellow)),
                        Severity::Info => Span::styled("🔵 Info", Style::default().fg(Color::Blue)),
                    };
                    
                    let mut description = rule.description.clone();
                    if state.search_active && !state.search_input.value().is_empty() {
                        // Highlight search matches
                        let search_term = state.search_input.value().to_lowercase();
                        description = description.replace(&search_term, &format!("»{}«", &search_term));
                    }
                    
                    let line = Line::from(vec![
                        Span::raw(format!("  {} ", checkbox)),
                        Span::raw(format!("{:<50} ", description)),
                        severity_badge,
                    ]);
                    
                    let mut style = Style::default();
                    if is_focused {
                        style = style.bg(Color::DarkGray);
                    }
                    if is_selected {
                        style = style.fg(Color::Green);
                    }
                    
                    items.push(ListItem::new(line).style(style));
                    line_to_index.push(Some(*idx));
                    current_line += 1;
                }
            }
        }
    }
    
    // Render the list
    let list = List::new(items)
        .block(Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM));
    
    frame.render_widget(list, area);
    
    // Render scrollbar
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    
    let mut scrollbar_state = ScrollbarState::new(current_line)
        .position(state.selected_index);
    
    frame.render_stateful_widget(
        scrollbar,
        area,
        &mut scrollbar_state,
    );
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::raw(" "),
        Span::styled("↑↓", Style::default().fg(Color::Cyan)),
        Span::raw(" Move  "),
        Span::styled("•", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" Select  "),
        Span::styled("•", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("Space", Style::default().fg(Color::Cyan)),
        Span::raw(" Continue  "),
        Span::styled("•", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" Back"),
    ]);
    
    let help = Paragraph::new(help_text)
        .style(Style::default().bg(Color::DarkGray));
    
    frame.render_widget(help, area);
}

fn render_edit_modal(frame: &mut Frame, area: Rect, rule: &ExtractedRule, form: &RuleEditForm) {
    use crate::cli::tui::init::modal::{centered_rect, render_modal_background};
    use ratatui::widgets::{Clear, BorderType};
    
    // Calculate modal area
    let modal_area = centered_rect(70, 60, area);
    
    // Clear background for modal
    render_modal_background(frame, modal_area);
    frame.render_widget(Clear, modal_area);
    
    // Create modal block
    let modal_block = Block::default()
        .title(" Edit Rule ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));
    
    let inner = modal_block.inner(modal_area);
    frame.render_widget(modal_block, modal_area);
    
    // Layout for form fields
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Description
            Constraint::Length(1),      // Spacing
            Constraint::Length(1),      // Severity
            Constraint::Length(1),      // Category
            Constraint::Length(1),      // When
            Constraint::Length(1),      // Spacing
            Constraint::Length(2),      // Block on violation
            Constraint::Length(1),      // Spacing
            Constraint::Length(1),      // Help
        ])
        .split(inner);
    
    // Description field
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if form.current_field == FormField::Description {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });
    
    let desc_paragraph = Paragraph::new(Line::from(vec![
        Span::raw(form.description.value()),
        if form.current_field == FormField::Description {
            Span::styled("█", Style::default().fg(Color::White))
        } else {
            Span::raw("")
        },
    ]))
    .block(desc_block);
    
    frame.render_widget(
        Paragraph::new("Description:"),
        Rect { y: chunks[0].y, ..chunks[0] }
    );
    frame.render_widget(desc_paragraph, Rect { y: chunks[0].y + 1, height: 2, ..chunks[0] });
    
    // Severity dropdown
    let severity_text = match form.severity {
        Severity::Critical => "🔴 Critical",
        Severity::Warning => "🟡 Warning",
        Severity::Info => "🔵 Info",
    };
    render_field(frame, chunks[2], "Severity:", severity_text, form.current_field == FormField::Severity);
    
    // Category field
    render_field(frame, chunks[3], "Category:", &form.category, form.current_field == FormField::Category);
    
    // When field
    render_field(frame, chunks[4], "When:", &form.when, form.current_field == FormField::When);
    
    // Block on violation checkbox
    let block_line = Line::from(vec![
        if form.block_on_violation {
            Span::raw("☑ Block action on violation")
        } else {
            Span::raw("☐ Block action on violation")
        },
    ]);
    let block_style = if form.current_field == FormField::BlockOnViolation {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    frame.render_widget(
        Paragraph::new(vec![
            block_line,
            Line::from(if form.block_on_violation { "☐ Warn only" } else { "☑ Warn only" }),
        ]).style(block_style),
        chunks[6]
    );
    
    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
        Span::raw(" Next field  "),
        Span::styled("[Ctrl+Enter]", Style::default().fg(Color::Cyan)),
        Span::raw(" Save  "),
        Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
        Span::raw(" Cancel"),
    ]))
    .alignment(ratatui::layout::Alignment::Center);
    
    frame.render_widget(help, chunks[8]);
}

fn render_field(frame: &mut Frame, area: Rect, label: &str, value: &str, is_focused: bool) {
    let line = Line::from(vec![
        Span::raw(format!("{:<12}", label)),
        Span::styled(
            format!("[{:<20}▼]", value),
            if is_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            }
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}