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
            Constraint::Min(5),         // Table
            Constraint::Length(2),      // Overall progress
            Constraint::Length(3),      // Compilation status
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
    
    // Compilation status
    render_compilation_status(frame, chunks[3], state);
    
    // Tip
    render_tip(frame, chunks[4], state);
    
    // Help bar
    render_help(frame, chunks[5]);
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
            TaskStatus::InProgress => {
                // Use same animation frame as in status text
                let spinner_frames = vec!["⟳", "⟲", "⟴", "⟵", "⟶", "⟷"];
                let frame_idx = (task.elapsed_ms / 200) as usize % spinner_frames.len();
                spinner_frames[frame_idx]
            },
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
            TaskStatus::InProgress => {
                // Animate the spinner based on elapsed time
                let spinner_frames = vec!["⟳", "⟲", "⟴", "⟵", "⟶", "⟷"];
                let frame_idx = (task.elapsed_ms / 200) as usize % spinner_frames.len();
                format!("{} Extracting...", spinner_frames[frame_idx])
            },
            TaskStatus::Complete => "Complete".to_string(),
            TaskStatus::Failed(err) => format!("Failed: {}", err),
        };
        
        let time_text = if task.elapsed_ms > 0 {
            if task.elapsed_ms < 1000 {
                format!("{}ms", task.elapsed_ms)
            } else {
                format!("{:.1}s", task.elapsed_ms as f64 / 1000.0)
            }
        } else {
            "--".to_string()
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
            format!("{} {}", status_icon, status_text),
            time_text,
            rules_text,
        ])
        .style(Style::default().fg(status_color))
    }).collect();
    
    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(40),
            Constraint::Percentage(35),
            Constraint::Length(10),
            Constraint::Length(8),
        ]
    )
    .header(headers)
    .block(Block::default().borders(Borders::TOP));
    
    frame.render_widget(table, area);
}

fn render_overall_progress(frame: &mut Frame, area: Rect, state: &ExtractionState) {
    let completed = state.tasks.iter()
        .filter(|t| matches!(t.status, TaskStatus::Complete))
        .count();
    let total = state.tasks.len();
    let total_rules: usize = state.extracted_rules.len();
    
    // Show progress as simple text, no gauge
    let progress_text = if completed == total {
        format!("✓ Extraction complete: {} rules found from {} files", total_rules, total)
    } else {
        format!("Extracting rules from {} files... {} rules found so far", total, total_rules)
    };
    
    let paragraph = Paragraph::new(progress_text)
        .block(Block::default().borders(Borders::TOP))
        .style(Style::default().fg(if completed == total { Color::Green } else { Color::White }));
    
    frame.render_widget(paragraph, area);
}

fn render_tip(frame: &mut Frame, area: Rect, state: &ExtractionState) {
    // Check if all tasks are complete
    let all_complete = state.tasks.iter()
        .all(|t| matches!(t.status, TaskStatus::Complete | TaskStatus::Failed(_)));
    
    let tip_text = if all_complete {
        vec![
            Line::from(vec![
                Span::styled("✓ All files processed! ", Style::default().fg(Color::Green)),
                Span::raw("Press "),
                Span::styled("Enter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" to continue to rule review."),
            ]),
        ]
    } else if state.custom_instructions.is_some() {
        vec![
            Line::from("💡 Using custom extraction instructions"),
        ]
    } else {
        vec![
            Line::from("💡 Using default extraction settings"),
        ]
    };
    
    let tip = Paragraph::new(tip_text)
        .style(Style::default().fg(Color::Yellow))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(tip, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::raw(" "),
        Span::styled("Extracting rules...", Style::default().fg(Color::Yellow)),
        Span::raw("  Press "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" to continue when all files complete  •  "),
        Span::styled("Esc or Q", Style::default().fg(Color::Cyan)),
        Span::raw(" Exit"),
    ]);
    
    let help = Paragraph::new(help_text)
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(help, area);
}

fn render_compilation_status(frame: &mut Frame, area: Rect, state: &ExtractionState) {
    let all_complete = state.tasks.iter()
        .all(|t| matches!(t.status, TaskStatus::Complete | TaskStatus::Failed(_)));
    
    if all_complete && !state.extracted_rules.is_empty() {
        // Get current time for compilation elapsed calculation
        
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let compile_elapsed = if state.compilation_started_at > 0 {
            current_time - state.compilation_started_at
        } else {
            0
        };
        
        let compile_time_str = if compile_elapsed < 1000 {
            format!("{}ms", compile_elapsed)
        } else {
            format!("{:.1}s", compile_elapsed as f64 / 1000.0)
        };
        
        // Animate the compilation spinner
        let spinner_frames = vec!["⟳", "⟲", "⟴", "⟵", "⟶", "⟷"];
        let frame_idx = (compile_elapsed / 200) as usize % spinner_frames.len();
        let spinner = spinner_frames[frame_idx];
        
        // Show compilation step
        let content = vec![
            Line::from(""),  // Empty line for spacing
            Line::from("─".repeat(area.width.min(80) as usize)),
            Line::from(vec![
                Span::raw("Compiling single rule set"),
                Span::raw("                                     "),
                Span::styled(format!("{} Compiling...", spinner), Style::default().fg(Color::Yellow)),
                Span::raw("     "),
                Span::raw(compile_time_str),
            ]),
        ];
        
        let paragraph = Paragraph::new(content);
        frame.render_widget(paragraph, area);
    }
}

