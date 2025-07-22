//! Success summary screen

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use crate::cli::tui::init::state::SuccessState;

pub fn render(frame: &mut Frame, state: &SuccessState) {
    // Main container with success styling
    let main_block = Block::default()
        .title(" ✅ Setup Complete! ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));
    
    let inner = main_block.inner(frame.area());
    frame.render_widget(main_block, frame.area());
    
    // Layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Success message
            Constraint::Length(1),      // Spacing
            Constraint::Length(11),     // Summary table
            Constraint::Length(1),      // Spacing
            Constraint::Length(6),      // Try commands
            Constraint::Min(1),         // Flexible space
            Constraint::Length(1),      // Help bar
        ])
        .split(inner);
    
    // Success message
    let success_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Your AI coding agent now has "),
            Span::styled("deterministic guardrails", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("!"),
        ]),
    ];
    let success_paragraph = Paragraph::new(success_text)
        .alignment(Alignment::Left);
    frame.render_widget(success_paragraph, chunks[0]);
    
    // Summary table
    render_summary_table(frame, chunks[2], state);
    
    // Try commands section
    render_try_commands(frame, chunks[4]);
    
    // Help bar
    render_help_bar(frame, chunks[6]);
}

fn render_summary_table(frame: &mut Frame, area: Rect, state: &SuccessState) {
    // Create the table data
    let rows = vec![
        Row::new(vec![
            Cell::from("Total Rules"),
            Cell::from(format!("{} (from {} sources)", 
                state.total_rules,
                if state.total_rules > 0 { "4" } else { "0" }
            )),
        ]),
        Row::new(vec![
            Cell::from("Critical (blocking)"),
            Cell::from(format!("{} rules - will halt operations", state.critical_count))
                .style(Style::default().fg(Color::Red)),
        ]),
        Row::new(vec![
            Cell::from("Warning (advisory)"),
            Cell::from(format!("{} rules - will show warnings", state.warning_count))
                .style(Style::default().fg(Color::Yellow)),
        ]),
        Row::new(vec![
            Cell::from("Info (logging only)"),
            Cell::from(format!("{} rules - tracked for metrics", state.info_count))
                .style(Style::default().fg(Color::Blue)),
        ]),
        Row::new(vec![
            Cell::from("Performance Impact"),
            Cell::from("~8ms average per file operation")
                .style(Style::default().fg(Color::Green)),
        ]),
        Row::new(vec![
            Cell::from("Config Location"),
            Cell::from(state.config_location.display().to_string())
                .style(Style::default().fg(Color::Cyan)),
        ]),
    ];
    
    let table = Table::new(
        rows,
        vec![
            Constraint::Length(22),
            Constraint::Min(40),
        ]
    )
    .block(
        Block::default()
            .title(" Configuration Summary ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
    )
    .column_spacing(3);
    
    frame.render_widget(table, area);
}

fn render_try_commands(frame: &mut Frame, area: Rect) {
    let commands = vec![
        Line::from("  Try these commands:"),
        Line::from(""),
    ];
    
    let header = Paragraph::new(commands);
    frame.render_widget(header, Rect { x: area.x, y: area.y, width: area.width, height: 2 });
    
    // Command box
    let command_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width.saturating_sub(4),
        height: 3,
    };
    
    let command_lines = vec![
        Line::from(vec![
            Span::raw(" $ "),
            Span::styled("cupcake test", Style::default().fg(Color::Green)),
            Span::raw("              # Validate your configuration"),
        ]),
        Line::from(vec![
            Span::raw(" $ "),
            Span::styled("cupcake status", Style::default().fg(Color::Green)),
            Span::raw("            # View active policies"),
        ]),
        Line::from(vec![
            Span::raw(" $ "),
            Span::styled("echo \"TODO: fix\" > test.py", Style::default().fg(Color::Green)),
            Span::raw(" # See Cupcake in action!"),
        ]),
    ];
    
    let commands_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    
    let commands_paragraph = Paragraph::new(command_lines)
        .block(commands_block)
        .style(Style::default().bg(Color::Black));
    
    frame.render_widget(commands_paragraph, command_area);
}

fn render_help_bar(frame: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::raw(" "),
        Span::styled("✓ Success!", Style::default().fg(Color::Green)),
        Span::raw("  Press "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" to exit"),
    ]);
    
    let help = Paragraph::new(help_text)
        .style(Style::default().bg(Color::DarkGray));
    
    frame.render_widget(help, area);
}