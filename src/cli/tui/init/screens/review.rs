//! Rule review screen - clean table view

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
};
use crate::cli::tui::init::state::{ReviewState, Severity, ExtractedRule};

pub fn render(frame: &mut Frame, state: &ReviewState) {
    // Main container
    let main_block = Block::default()
        .title(format!(" Review Extracted Rules ({} of {} selected) ", state.selected.len(), state.rules.len()))
        .borders(Borders::ALL);
    
    let inner = main_block.inner(frame.area());
    frame.render_widget(main_block, frame.area());
    
    // Layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Header
            Constraint::Min(10),        // Rule table
            Constraint::Length(3),      // Status
            Constraint::Length(1),      // Help
        ])
        .split(inner);
    
    render_header(frame, chunks[0], state);
    render_rule_table(frame, chunks[1], state);
    render_status(frame, chunks[2], state);
    render_help(frame, chunks[3]);
}

fn render_header(frame: &mut Frame, area: Rect, _state: &ReviewState) {
    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Review the extracted rules below. "),
            Span::styled("Selected rules will be converted to Cupcake policies.", Style::default().fg(Color::Green)),
        ]),
    ];
    
    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, area);
}

fn render_rule_table(frame: &mut Frame, area: Rect, state: &ReviewState) {
    // Sort rules by severity (High -> Medium -> Low) and then by ID
    let mut sorted_rules: Vec<(usize, &ExtractedRule)> = state.rules.iter()
        .enumerate()
        .collect();
    
    sorted_rules.sort_by(|a, b| {
        let severity_order = |s: &Severity| match s {
            Severity::High => 0,
            Severity::Medium => 1,
            Severity::Low => 2,
        };
        
        match severity_order(&a.1.severity).cmp(&severity_order(&b.1.severity)) {
            std::cmp::Ordering::Equal => a.1.id.cmp(&b.1.id),
            other => other,
        }
    });
    
    // Table headers with padding
    let headers = Row::new(vec![
        "  #",
        "Rule",
        "Hook Action",
        "Severity",
        "Rationale",
        "Source",
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .bottom_margin(1);
    
    // Table rows
    let rows: Vec<Row> = sorted_rules.iter().enumerate().map(|(display_idx, (actual_idx, rule))| {
        let is_selected = state.selected.contains(actual_idx);
        let is_focused = state.selected_index == display_idx;
        
        // Checkbox with number
        let checkbox = if is_selected { "[✓]" } else { "[ ]" };
        let number = format!("  {} {}", checkbox, display_idx + 1);
        
        // Truncate long descriptions for table display
        let rule_desc = if rule.description.len() > 30 {
            format!("{}...", &rule.description[..27])
        } else {
            rule.description.clone()
        };
        
        let hook_desc = if rule.hook_description.len() > 35 {
            format!("{}...", &rule.hook_description[..32])
        } else {
            rule.hook_description.clone()
        };
        
        // Severity with color
        let severity_text = match rule.severity {
            Severity::High => "High",
            Severity::Medium => "Medium",
            Severity::Low => "Low",
        };
        
        // Truncate rationale
        let rationale = if rule.policy_decision.rationale.len() > 40 {
            format!("{}...", &rule.policy_decision.rationale[..37])
        } else {
            rule.policy_decision.rationale.clone()
        };
        
        // Get relative path for source
        let source_path = if rule.source_file.is_absolute() {
            std::env::current_dir()
                .ok()
                .and_then(|cwd| rule.source_file.strip_prefix(cwd).ok())
                .unwrap_or(&rule.source_file)
        } else {
            &rule.source_file
        };
        
        let source_str = source_path.to_string_lossy();
        let source_display = if source_str.len() > 20 {
            format!("...{}", &source_str[source_str.len()-17..])
        } else {
            source_str.to_string()
        };
        
        // Style based on selection and focus
        let row_color = if is_selected {
            Color::Green
        } else {
            match rule.severity {
                Severity::High => Color::Red,
                Severity::Medium => Color::Yellow,
                Severity::Low => Color::Blue,
            }
        };
        
        let row_style = if is_focused {
            Style::default().fg(row_color).bg(Color::DarkGray)
        } else {
            Style::default().fg(row_color)
        };
        
        Row::new(vec![
            number,
            rule_desc,
            hook_desc,
            severity_text.to_string(),
            rationale,
            source_display,
        ])
        .style(row_style)
    }).collect();
    
    let table = Table::new(
        rows,
        &[
            Constraint::Length(8),      // # with checkbox
            Constraint::Percentage(20), // Rule
            Constraint::Percentage(25), // Hook Action
            Constraint::Length(8),      // Severity
            Constraint::Percentage(30), // Rationale
            Constraint::Percentage(17), // Source
        ]
    )
    .header(headers)
    .block(Block::default().borders(Borders::TOP));
    
    frame.render_widget(table, area);
}

fn render_status(frame: &mut Frame, area: Rect, state: &ReviewState) {
    let high_count = state.rules.iter().filter(|r| matches!(r.severity, Severity::High)).count();
    let medium_count = state.rules.iter().filter(|r| matches!(r.severity, Severity::Medium)).count();
    let low_count = state.rules.iter().filter(|r| matches!(r.severity, Severity::Low)).count();
    
    let selected_high = state.selected.iter()
        .filter_map(|idx| state.rules.get(*idx))
        .filter(|r| matches!(r.severity, Severity::High))
        .count();
    let selected_medium = state.selected.iter()
        .filter_map(|idx| state.rules.get(*idx))
        .filter(|r| matches!(r.severity, Severity::Medium))
        .count();
    let selected_low = state.selected.iter()
        .filter_map(|idx| state.rules.get(*idx))
        .filter(|r| matches!(r.severity, Severity::Low))
        .count();
    
    let status_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("🔴 High: {}/{}", selected_high, high_count), Style::default().fg(Color::Red)),
            Span::raw("    "),
            Span::styled(format!("🟡 Medium: {}/{}", selected_medium, medium_count), Style::default().fg(Color::Yellow)),
            Span::raw("    "),
            Span::styled(format!("🔵 Low: {}/{}", selected_low, low_count), Style::default().fg(Color::Blue)),
            Span::raw("    "),
            if state.selected.is_empty() {
                Span::styled("Select at least one rule to continue", Style::default().fg(Color::DarkGray))
            } else {
                Span::styled("Press Space to continue", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            },
        ]),
    ];
    
    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::TOP));
    
    frame.render_widget(status, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::raw(" "),
        Span::styled("↑↓", Style::default().fg(Color::Cyan)),
        Span::raw(" Navigate  "),
        Span::styled("•", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" Toggle selection  "),
        Span::styled("•", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("a", Style::default().fg(Color::Cyan)),
        Span::raw(" Select all  "),
        Span::styled("•", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("n", Style::default().fg(Color::Cyan)),
        Span::raw(" Select none  "),
        Span::styled("•", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("Space", Style::default().fg(Color::Cyan)),
        Span::raw(" Continue  "),
        Span::styled("•", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("Esc or Q", Style::default().fg(Color::Cyan)),
        Span::raw(" Exit"),
    ]);
    
    let help = Paragraph::new(help_text)
        .style(Style::default().bg(Color::DarkGray));
    
    frame.render_widget(help, area);
}