//! Rule extraction progress screen

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
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
            Constraint::Min(10),        // Table (including compilation row)
            Constraint::Length(4),      // Tip (with more padding)
            Constraint::Length(1),      // Help
        ])
        .split(inner);
    
    // Header
    render_header(frame, chunks[0], state);
    
    // Task table
    render_task_table(frame, chunks[1], state);
    
    // Tip
    render_tip(frame, chunks[2], state);
    
    // Help bar
    render_help(frame, chunks[3]);
}

fn render_header(frame: &mut Frame, area: Rect, state: &ExtractionState) {
    let file_count = state.tasks.len();
    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),  // Left padding to align with table
            Span::raw(format!("Processing {} files in parallel...", file_count)),
            Span::raw("           "),
            Span::styled("Sonnet 4", Style::default().fg(Color::Cyan)),
        ]),
    ];
    
    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, area);
}

fn render_task_table(frame: &mut Frame, area: Rect, state: &ExtractionState) {
    // Table headers with padding
    let headers = Row::new(vec![
        "  #",  // Number column with padding
        "File",
        "Status",
        "Time",
        "Rules",
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .bottom_margin(1);
    
    // Table rows
    let mut rows: Vec<Row> = state.tasks.iter().enumerate().map(|(idx, task)| {
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
        
        let number = format!("  {}.", idx + 1);
        
        Row::new(vec![
            number,
            task.file_name.clone(),
            format!("{} {}", status_icon, status_text),
            time_text,
            rules_text,
        ])
        .style(Style::default().fg(status_color))
    }).collect();
    
    // Check if all tasks are complete
    let all_complete = state.tasks.iter()
        .all(|t| matches!(t.status, TaskStatus::Complete | TaskStatus::Failed(_)));
    
    // Add separator row
    if !state.tasks.is_empty() {
        rows.push(Row::new(vec![
            "  ─".to_string(),  // Separator for number column
            "─".repeat(40),
            "─".repeat(35),
            "─".repeat(10),
            "─".repeat(8),
        ]).style(Style::default().fg(Color::DarkGray)));
    }
    
    // Add compilation row
    if !state.tasks.is_empty() {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let compile_elapsed = if state.compilation_started_at > 0 {
            if state.compilation_complete && state.compilation_completed_at > 0 {
                // Use the final completion time
                state.compilation_completed_at - state.compilation_started_at
            } else {
                // Still running
                current_time - state.compilation_started_at
            }
        } else {
            0
        };
        
        let (compile_status_text, compile_time_text, compile_color) = if !all_complete {
            // Still extracting - show grayed out
            ("Waiting...".to_string(), "--".to_string(), Color::DarkGray)
        } else if state.compilation_complete {
            // Complete
            let time_str = if compile_elapsed < 1000 {
                format!("{}ms", compile_elapsed)
            } else {
                format!("{:.1}s", compile_elapsed as f64 / 1000.0)
            };
            (
                "✓ Complete".to_string(),
                time_str,
                Color::Green
            )
        } else if compile_elapsed > 0 {
            // Compiling
            let spinner_frames = vec!["⟳", "⟲", "⟴", "⟵", "⟶", "⟷"];
            let frame_idx = (compile_elapsed / 200) as usize % spinner_frames.len();
            let time_str = if compile_elapsed < 1000 {
                format!("{}ms", compile_elapsed)
            } else {
                format!("{:.1}s", compile_elapsed as f64 / 1000.0)
            };
            (
                format!("{} Compiling...", spinner_frames[frame_idx]),
                time_str,
                Color::Yellow
            )
        } else {
            // Just started
            ("Starting...".to_string(), "--".to_string(), Color::Yellow)
        };
        
        // Show compiled rule count when complete
        let compile_rules_text = if state.compilation_complete && state.compiled_rule_count > 0 {
            state.compiled_rule_count.to_string()
        } else {
            "--".to_string()
        };
        
        rows.push(Row::new(vec![
            "    ".to_string(),  // Empty space for number column
            "Compile to single rule set".to_string(),
            compile_status_text,
            compile_time_text,
            compile_rules_text,
        ]).style(Style::default().fg(compile_color)));
    }
    
    let table = Table::new(
        rows,
        &[
            Constraint::Length(5),      // Number column
            Constraint::Percentage(30), // File (reduced from 40%)
            Constraint::Percentage(25), // Status
            Constraint::Length(10),     // Time
            Constraint::Length(8),      // Rules
        ]
    )
    .header(headers)
    .block(Block::default().borders(Borders::TOP));
    
    frame.render_widget(table, area);
}

fn render_tip(frame: &mut Frame, area: Rect, state: &ExtractionState) {
    // Check if all tasks are complete
    let all_complete = state.tasks.iter()
        .all(|t| matches!(t.status, TaskStatus::Complete | TaskStatus::Failed(_)));
    
    // Use compiled rule count if compilation is complete
    let final_rule_count = if state.compilation_complete && state.compiled_rule_count > 0 {
        state.compiled_rule_count
    } else {
        state.extracted_rules.len()
    };
    
    let tip_text = if all_complete && state.compilation_complete {
        vec![
            Line::from(""),  // Empty line for padding
            Line::from(vec![
                Span::styled(format!("✓ {} rules compiled! ", final_rule_count), Style::default().fg(Color::Green)),
                Span::raw("Press "),
                Span::styled("Enter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" to continue to rule review."),
            ]),
            Line::from(""),  // Extra padding at bottom
        ]
    } else if all_complete && !state.extracted_rules.is_empty() {
        vec![
            Line::from(""),  // Empty line for padding
            Line::from("⟳ Compiling rules into single set..."),
            Line::from(""),  // Extra padding at bottom
        ]
    } else if state.custom_instructions.is_some() {
        vec![
            Line::from(""),  // Empty line for padding
            Line::from("Using custom extraction instructions"),
            Line::from(""),  // Extra padding at bottom
        ]
    } else {
        vec![
            Line::from(""),  // Empty line for padding
            Line::from("Determining which rules to convert to hooks..."),
            Line::from(""),  // Extra padding at bottom
        ]
    };
    
    let tip = Paragraph::new(tip_text)
        .block(Block::default().borders(Borders::TOP))
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

