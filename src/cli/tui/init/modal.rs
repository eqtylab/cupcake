//! Modal overlay system for the TUI

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

/// Calculate centered modal area
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Render a modal background (dimmed overlay)
pub fn render_modal_background(frame: &mut Frame, area: Rect) {
    // Clear the area to create overlay effect
    frame.render_widget(Clear, area);
    
    // Optionally, we could render a semi-transparent overlay here
    // For now, just clearing is enough
}

/// Render custom instructions modal
pub fn render_custom_instructions_modal(
    frame: &mut Frame, 
    area: Rect,
    input_text: &str,
    cursor_position: usize,
) {
    // Calculate modal area (60% width, 50% height)
    let modal_area = centered_rect(60, 50, area);
    
    // Clear background for modal
    render_modal_background(frame, modal_area);
    
    // Create modal block
    let modal_block = Block::default()
        .title(" Custom Instructions (Optional) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));
    
    let inner_area = modal_block.inner(modal_area);
    frame.render_widget(modal_block, modal_area);
    
    // Split inner area for content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),      // Instructions
            Constraint::Length(1),      // Spacing
            Constraint::Min(5),         // Text input area
            Constraint::Length(1),      // Spacing
            Constraint::Length(2),      // Hint
            Constraint::Length(1),      // Help text
        ])
        .split(inner_area);
    
    // Instructions
    let instructions = Paragraph::new("Add context for rule extraction:")
        .style(Style::default().fg(Color::White));
    frame.render_widget(instructions, chunks[0]);
    
    // Text input area with border
    let input_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::DarkGray));
    
    let input_inner = input_block.inner(chunks[2]);
    frame.render_widget(input_block, chunks[2]);
    
    // Render text with cursor
    let text = if input_text.is_empty() {
        vec![Line::from(vec![
            Span::raw(""),
            Span::styled("█", Style::default().fg(Color::White)),
        ])]
    } else {
        // Split text into lines for wrapping
        let mut lines = Vec::new();
        let text_before_cursor = &input_text[..cursor_position.min(input_text.len())];
        let text_after_cursor = &input_text[cursor_position.min(input_text.len())..];
        
        // Simple line wrapping (can be improved)
        let line = Line::from(vec![
            Span::raw(text_before_cursor),
            Span::styled("█", Style::default().fg(Color::White)),
            Span::raw(text_after_cursor),
        ]);
        lines.push(line);
        
        lines
    };
    
    let input_paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: false });
    frame.render_widget(input_paragraph, input_inner);
    
    // Hint
    let hint = Paragraph::new("💡 Helps AI understand your conventions")
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(hint, chunks[4]);
    
    // Help text
    let help_text = Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
        Span::raw(" Apply  "),
        Span::styled("[s]", Style::default().fg(Color::Cyan)),
        Span::raw(" Skip  "),
        Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
        Span::raw(" Cancel"),
    ]);
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[5]);
}