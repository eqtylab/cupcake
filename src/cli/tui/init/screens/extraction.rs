//! Rule extraction progress screen

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table},
};
use crate::cli::tui::init::state::{ExtractionState, TaskStatus};

pub fn render(frame: &mut Frame, state: &ExtractionState) {
    // Main container
    let main_block = Block::default()
        .title(" Analyzing your files for security rules... ")
        .borders(Borders::ALL);
    
    let inner = main_block.inner(frame.area());
    frame.render_widget(main_block, frame.area());
    
    // Layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Header
            Constraint::Min(10),        // Table
            Constraint::Length(3),      // Overall progress
            Constraint::Length(2),      // Tip
            Constraint::Length(1),      // Help
        ])
        .split(inner);
    
    // Header
    render_header(frame, chunks[0], state);
    
    // Task table
    render_task_table(frame, chunks[1], state);
    
    // Overall progress
    render_overall_progress(frame, chunks[2], state);
    
    // Tip
    render_tip(frame, chunks[3], state);
    
    // Help bar
    render_help(frame, chunks[4]);
}

fn render_header(frame: &mut Frame, area: Rect, state: &ExtractionState) {
    let file_count = state.tasks.len();
    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw(format!("Processing {} files in parallel...", file_count)),
            Span::raw("           "),
            Span::styled("Sonnet 4", Style::default().fg(Color::Cyan)),
        ]),
    ];
    
    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, area);
}

fn render_task_table(frame: &mut Frame, area: Rect, state: &ExtractionState) {
    // Table headers
    let headers = Row::new(vec![
        "File",
        "Progress",
        "Status",
        "Time",
        "Rules",
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .bottom_margin(1);
    
    // Table rows
    let rows: Vec<Row> = state.tasks.iter().map(|task| {
        let status_icon = match &task.status {
            TaskStatus::Queued => "⏳",
            TaskStatus::InProgress => "⟳",
            TaskStatus::Complete => "✓",
            TaskStatus::Failed(_) => "✗",
        };
        
        let status_color = match &task.status {
            TaskStatus::Queued => Color::DarkGray,
            TaskStatus::InProgress => Color::Yellow,
            TaskStatus::Complete => Color::Green,
            TaskStatus::Failed(_) => Color::Red,
        };
        
        let status_text = match &task.status {
            TaskStatus::Queued => "Queued".to_string(),
            TaskStatus::InProgress => format!("Extract {:.0}%", task.progress * 100.0),
            TaskStatus::Complete => "Complete".to_string(),
            TaskStatus::Failed(err) => format!("Failed: {}", err),
        };
        
        let time_text = if task.elapsed_ms > 0 {
            format!("{}ms", task.elapsed_ms)
        } else {
            "--ms".to_string()
        };
        
        let rules_text = match &task.status {
            TaskStatus::Complete => task.rules_found.to_string(),
            TaskStatus::InProgress if task.rules_found > 0 => {
                format!("{}/--", task.rules_found)
            }
            _ => "--".to_string(),
        };
        
        Row::new(vec![
            task.file_name.clone(),
            format!("{:<12}", create_progress_bar(task.progress)),
            format!("{} {}", status_icon, status_text),
            time_text,
            rules_text,
        ])
        .style(Style::default().fg(status_color))
    }).collect();
    
    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(35),
            Constraint::Length(14),
            Constraint::Percentage(25),
            Constraint::Length(8),
            Constraint::Length(8),
        ]
    )
    .header(headers)
    .block(Block::default().borders(Borders::TOP));
    
    frame.render_widget(table, area);
}

fn render_overall_progress(frame: &mut Frame, area: Rect, state: &ExtractionState) {
    let _completed = state.tasks.iter()
        .filter(|t| matches!(t.status, TaskStatus::Complete))
        .count();
    let _total = state.tasks.len();
    let total_rules: usize = state.tasks.iter()
        .filter(|t| matches!(t.status, TaskStatus::Complete))
        .map(|t| t.rules_found)
        .sum();
    
    let progress_text = format!("{} of {} rules extracted", total_rules, total_rules + 21); // Mock total
    
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::TOP))
        .gauge_style(Style::default().fg(Color::Green))
        .percent((state.overall_progress * 100.0) as u16)
        .label(format!("Overall: {:.0}%        {}", state.overall_progress * 100.0, progress_text));
    
    frame.render_widget(gauge, area);
}

fn render_tip(frame: &mut Frame, area: Rect, state: &ExtractionState) {
    let tip_text = if state.custom_instructions.is_some() {
        "💡 Tip: Extraction uses your custom instructions"
    } else {
        "💡 Tip: Using default extraction settings"
    };
    
    let tip = Paragraph::new(tip_text)
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(tip, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::raw(" "),
        Span::styled("Processing...", Style::default().fg(Color::Yellow)),
        Span::raw("  Press "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" when complete  •  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" Exit"),
    ]);
    
    let help = Paragraph::new(help_text)
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(help, area);
}

fn create_progress_bar(progress: f64) -> String {
    let filled = (progress * 12.0) as usize;
    let empty = 12 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}