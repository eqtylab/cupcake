//! Compilation and sync progress screen

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};
use crate::cli::tui::init::state::{CompilationState, PhaseStatus};

pub fn render(frame: &mut Frame, state: &CompilationState) {
    // Main container
    let main_block = Block::default()
        .title(" Creating your security policies... ")
        .borders(Borders::ALL);
    
    let inner = main_block.inner(frame.area());
    frame.render_widget(main_block, frame.area());
    
    // Layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),      // Spacing
            Constraint::Length(2),      // Overall progress
            Constraint::Length(1),      // Spacing
            Constraint::Min(10),        // Phase details
            Constraint::Length(8),      // Log viewer
            Constraint::Length(1),      // Help
        ])
        .split(inner);
    
    // Overall progress bar
    render_overall_progress(frame, chunks[1], state);
    
    // Phase details
    render_phase_details(frame, chunks[3], state);
    
    // Log viewer
    render_log_viewer(frame, chunks[4], state);
    
    // Help bar
    render_help(frame, chunks[5], state);
}

fn render_overall_progress(frame: &mut Frame, area: Rect, state: &CompilationState) {
    let current_phase_name = state.phases.get(state.current_phase)
        .map(|p| p.name.as_str())
        .unwrap_or("Initializing");
    
    let label = format!(
        "{}%  Phase {} of {}",
        (state.overall_progress * 100.0) as u8,
        state.current_phase + 1,
        state.phases.len()
    );
    
    let gauge = Gauge::default()
        .percent((state.overall_progress * 100.0) as u16)
        .label(label)
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black));
    
    frame.render_widget(gauge, area);
}

fn render_phase_details(frame: &mut Frame, area: Rect, state: &CompilationState) {
    let mut lines = Vec::new();
    
    for (idx, phase) in state.phases.iter().enumerate() {
        let status_icon = match &phase.status {
            PhaseStatus::Pending => "⏳",
            PhaseStatus::InProgress => "⟳",
            PhaseStatus::Complete => "✓",
            PhaseStatus::Failed(_) => "✗",
        };
        
        let status_color = match &phase.status {
            PhaseStatus::Pending => Color::DarkGray,
            PhaseStatus::InProgress => Color::Yellow,
            PhaseStatus::Complete => Color::Green,
            PhaseStatus::Failed(_) => Color::Red,
        };
        
        // Phase header
        let phase_line = Line::from(vec![
            Span::styled(status_icon, Style::default().fg(status_color)),
            Span::raw(" "),
            Span::styled(
                format!("Phase {}: {}", idx + 1, phase.name),
                Style::default()
                    .fg(status_color)
                    .add_modifier(if idx == state.current_phase { Modifier::BOLD } else { Modifier::empty() })
            ),
            if phase.elapsed_ms > 0 {
                Span::raw(format!("                              {}ms", phase.elapsed_ms))
            } else {
                Span::raw("")
            },
        ]);
        lines.push(ListItem::new(phase_line));
        
        // Phase details
        if !phase.details.is_empty() && (matches!(phase.status, PhaseStatus::InProgress | PhaseStatus::Complete) || matches!(phase.status, PhaseStatus::Failed(_))) {
            for detail in &phase.details {
                let detail_line = Line::from(vec![
                    Span::raw("   "),
                    if detail.starts_with("✓") {
                        Span::styled(detail.clone(), Style::default().fg(Color::Green))
                    } else if detail.starts_with("⟳") {
                        Span::styled(detail.clone(), Style::default().fg(Color::Yellow))
                    } else if detail.starts_with("✗") {
                        Span::styled(detail.clone(), Style::default().fg(Color::Red))
                    } else {
                        Span::raw(detail.clone())
                    },
                ]);
                lines.push(ListItem::new(detail_line));
            }
        }
        
        lines.push(ListItem::new("")); // Spacing between phases
    }
    
    let list = List::new(lines);
    frame.render_widget(list, area);
}

fn render_log_viewer(frame: &mut Frame, area: Rect, state: &CompilationState) {
    let title = if state.show_logs {
        " Installation Log ─────────────────────────────────────────[▼ Hide]─"
    } else {
        " Installation Log ─────────────────────────────────────────[▶ Show]─"
    };
    
    let log_block = Block::default()
        .title(title)
        .borders(Borders::ALL);
    
    let inner = log_block.inner(area);
    frame.render_widget(log_block, area);
    
    if state.show_logs {
        // Show last N logs that fit
        let visible_logs = state.logs.len().saturating_sub(inner.height as usize);
        let log_items: Vec<ListItem> = state.logs[visible_logs..]
            .iter()
            .map(|log| {
                let timestamp = "[12:34:01]"; // Mock timestamp
                ListItem::new(Line::from(vec![
                    Span::styled(timestamp, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::raw(log),
                ]))
            })
            .collect();
        
        let log_list = List::new(log_items);
        frame.render_widget(log_list, inner);
    }
}

fn render_help(frame: &mut Frame, area: Rect, state: &CompilationState) {
    let is_complete = state.phases.iter().all(|p| matches!(p.status, PhaseStatus::Complete));
    
    let help_text = if is_complete {
        Line::from(vec![
            Span::raw(" "),
            Span::styled("✓ Complete!", Style::default().fg(Color::Green)),
            Span::raw("  Press "),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" to continue"),
        ])
    } else {
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Creating policies...", Style::default().fg(Color::Yellow)),
            Span::raw("  Press "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" to exit"),
        ])
    };
    
    let help = Paragraph::new(help_text)
        .style(Style::default().bg(Color::DarkGray));
    
    frame.render_widget(help, area);
}