//! File discovery and selection screen

use std::collections::HashSet;
use std::path::PathBuf;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
};

use crate::cli::tui::init::state::{DiscoveryState, Pane, RuleFile};

/// Render the discovery screen
pub fn render(frame: &mut Frame, state: &DiscoveryState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());
    
    render_main_area(frame, chunks[0], state);
    render_help_bar(frame, chunks[1]);
    
    // Render custom prompt modal if active
    if state.show_custom_prompt {
        crate::cli::tui::init::modal::render_custom_instructions_modal(
            frame,
            frame.area(),
            state.custom_prompt_input.value(),
            state.custom_prompt_input.visual_cursor(),
        );
    }
}

/// Render the main area with file list and preview
fn render_main_area(frame: &mut Frame, area: Rect, state: &DiscoveryState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    
    render_file_list(frame, chunks[0], state);
    render_preview_pane(frame, chunks[1], state);
}

/// Render the file list pane
fn render_file_list(frame: &mut Frame, area: Rect, state: &DiscoveryState) {
    // Build list items
    let mut items: Vec<ListItem> = vec![];
    
    // Add scanning progress if not complete
    if !state.scan_complete {
        let progress_text = format!("Scanning repository... {:.0}%", state.scan_progress * 100.0);
        items.push(ListItem::new(Line::from(vec![
            Span::styled(progress_text, Style::default().fg(Color::Yellow)),
        ])));
        
        // Add progress gauge
        let gauge = Gauge::default()
            .percent((state.scan_progress * 100.0) as u16)
            .style(Style::default().fg(Color::Yellow));
        
        // Render gauge in a small area
        let gauge_area = Rect {
            x: area.x + 1,
            y: area.y + 2,
            width: area.width.saturating_sub(2),
            height: 1,
        };
        frame.render_widget(gauge, gauge_area);
        
        items.push(ListItem::new("")); // Empty line after gauge
    }
    
    // Add discovered files
    for (idx, file) in state.files.iter().enumerate() {
        let is_selected = state.selected.contains(&file.path);
        let is_focused = idx == state.selected_index;
        
        let checkbox = if is_selected { "[✓] " } else { "[ ] " };
        let badge = format!("[{}]", file.agent.as_str());
        
        let mut style = Style::default();
        if is_focused {
            style = style.bg(Color::DarkGray);
        }
        if is_selected {
            style = style.fg(Color::Green);
        }
        
        // Main file line - show relative path
        let display_path = if file.path.is_absolute() {
            std::env::current_dir()
                .ok()
                .and_then(|cwd| file.path.strip_prefix(cwd).ok())
                .unwrap_or(&file.path)
        } else {
            &file.path
        };
        
        let checkbox_span = if is_selected {
            Span::styled(checkbox, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(checkbox, Style::default().fg(Color::DarkGray))
        };
        
        let line = Line::from(vec![
            checkbox_span,
            Span::raw(format!("{:<30} ", display_path.display())),
            Span::styled(badge, Style::default().fg(Color::Cyan)),
        ]);
        
        items.push(ListItem::new(line).style(style));
        
        // If directory, show children
        if file.is_directory && !file.children.is_empty() {
            for child in &file.children {
                let child_name = child.file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                
                let child_line = Line::from(vec![
                    Span::raw("   ├─ "),
                    Span::raw(child_name.to_string()),
                ]);
                
                let mut child_style = Style::default().fg(Color::DarkGray);
                if is_selected {
                    child_style = child_style.fg(Color::Gray);
                }
                
                items.push(ListItem::new(child_line).style(child_style));
            }
        }
    }
    
    // Add status line if complete
    if state.scan_complete && !state.files.is_empty() {
        items.push(ListItem::new("")); // Empty line
        
        let selected_count = state.selected.len();
        if selected_count > 0 {
            let total_files = count_total_files(&state.files, &state.selected);
            let status = format!("Selected: {} sources ({} files)", selected_count, total_files);
            items.push(ListItem::new(Line::from(vec![
                Span::styled(status, Style::default().fg(Color::Magenta)),
            ])));
            
            // Add continue prompt
            items.push(ListItem::new(Line::from(vec![
                Span::styled("[ Press ", Style::default().fg(Color::Green)),
                Span::styled("Space", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" to continue ]", Style::default().fg(Color::Green)),
            ])));
        }
    }
    
    // Create list widget with helpful title
    let title = if state.selected.is_empty() {
        " Rule Files ".to_string()
    } else {
        let count = state.selected.len();
        format!(" {} selected ", count)
    };
    
    let list = List::new(items)
        .block(Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(if state.focused_pane == Pane::FileList {
                Style::default().fg(Color::Blue)
            } else {
                Style::default()
            }));
    
    frame.render_widget(list, area);
}

/// Render the preview pane
fn render_preview_pane(frame: &mut Frame, area: Rect, state: &DiscoveryState) {
    let title = if let Some(file) = state.files.get(state.selected_index) {
        let display_path = if file.path.is_absolute() {
            std::env::current_dir()
                .ok()
                .and_then(|cwd| file.path.strip_prefix(cwd).ok())
                .unwrap_or(&file.path)
        } else {
            &file.path
        };
        format!(" {} ", display_path.display())
    } else {
        " Preview ".to_string()
    };
    
    let content = state.preview_content.as_deref().unwrap_or(
        if state.files.is_empty() {
            "No files discovered yet..."
        } else {
            "Loading preview..."
        }
    );
    
    let preview = Paragraph::new(content)
        .block(Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(if state.focused_pane == Pane::Preview {
                Style::default().fg(Color::Blue)
            } else {
                Style::default()
            }))
        .wrap(Wrap { trim: true });
    
    frame.render_widget(preview, area);
}

/// Render the help bar
fn render_help_bar(frame: &mut Frame, area: Rect) {
    let help_text = vec![
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
        Span::raw(" Exit"),
    ];
    
    let help = Paragraph::new(Line::from(help_text))
        .style(Style::default().bg(Color::DarkGray));
    
    frame.render_widget(help, area);
}

/// Count total files including children
fn count_total_files(files: &[RuleFile], selected: &HashSet<PathBuf>) -> usize {
    let mut count = 0;
    for file in files {
        if selected.contains(&file.path) {
            if file.is_directory {
                count += file.children.len();
            } else {
                count += 1;
            }
        }
    }
    count
}