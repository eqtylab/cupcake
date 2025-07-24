//! Landing screen with ASCII art and introduction

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::cli::tui::init::state::LandingState;

const CUPCAKE_ASCII: &str = r#"                                                   ██▓                     ██                                                  
                                                ██████                     ▓███                                                
                                             ██▓█████▓                     ███▓█▓                                              
                                           ██▓█████████                    █▓█████                                             
                                          ██████▓██████                   ████ ████                                            
                                         ███████ ████████████████████████▓▓▓▓██████▓                                           
                                        █▓█████ ██████████████████▓███▓         ▓███                                           
                                       █████▓█ █████████████████████▓  ███         ██                                          
                                      ██ ██████████████████████████████             ██                                         
                                      ██ ▓▓███████████████████████████▓█              █                                        
                                      ▓██████████████████████████████████              ▓█                                      
                                      ██████████████████████████████████▓      ██        █                                     
                                      ███████████████████▓██▓████████▓███    █            ▓                                    
                                       ███████████████▓ █████████████ ███    █     ██     ██                                   
                                       █████████████▓ ██▓▓███▓▓█████▓██▓█     █     ███████▓                                   
                                       ██████████████████ █▓█████████▓████         █████  ▓█                                   
                                        ▓██████████████▓██████████████████                █▓                                   
                                        █████████████████▓████████████              ██   ▓█ █                                  
                                      █████▓██████████████████████████                ██ █  ▓▓                                 
                                     ██▓██████████████████████████▓         ███▓▓██████▓█   ██                                 
                                    ██████████████████████████████         ██████████████   ██                                 
                                 ███████████████████████████████▓          ██████ ███▓██▓ █ █▓                                 
                                █▓█████▓███████████████████████▓███         ████▓██▓██▓██ ███                                  
                              ███▓███████████████████████████████▓            █████▓███    ▓                                   
                              █ ██▓█████▓▓████████████████████████                ██       █▓                                  
                              ███▓▓██████ ▓██████████████████████▓██              █▓       ██                                  
                             ▓████▓▓▓██████ ▓███████▓███████████████          █▓██████▓   █▓                                   
                            ██▓▓▓▓███▓███████ ███████▓██████████████       ▓██████▓█████  ██                                   
                           ██████▓█▓█▓▓███████▓▓  ████▓█████████████████████▓        █████▓                                    
                          ███████▓█▓▓▓▓█████████████▓█ ███▓███████████████▓ ▓██     ▓█▓███                                     
                         ▓███████▓██▓█▓▓█▓█████████████▓ ▓█████████████▓▓▓█▓███████████                                        
                         █████████████▓██▓█▓██████████████        ▓█████▓█ ██████▓██▓                                          
                          ▓█████████▓█████▓▓▓▓██████████████ ████████████ █████▓ █████                                         
                            ██████████▓▓▓ ▓██▓██████████████▓█████████████████▓███▓▓███                                        
                              ██████████████▓▓▓▓███▓████████████████████████▓▓▓████████                                        
                               ███████████████▓██▓█▓██████████████████▓▓█████████▓██████                                       
                                ▓████████████▓▓█▓██▓███▓▓█▓█▓▓▓▓███▓██▓█████████████████                                       
                                  ██████████████▓█▓█▓█▓▓█▓▓█ ▓▓▓▓▓▓▓▓█▓▓▓██▓▓███████████                                       
                                    ▓████████████████████▓█▓▓▓▓▓▓████▓▓█████████████▓ ██                                       
                                      █▓███████████████████████████████████████████▓  █                                        
                                           ▓████████████████████████████████████▓█                                             
                                                 ▓████████████████████████████▓                                                
                                                         ▓█████████████▓                                                       "#;

pub fn render(frame: &mut Frame, state: &LandingState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),         // Main content
            Constraint::Length(1),      // Help bar
        ])
        .split(frame.area());

    // Check if terminal is large enough for ASCII art
    let ascii_width = 120;
    let ascii_height = 43;
    let show_ascii = frame.area().width >= ascii_width && frame.area().height >= (ascii_height + 15);

    if show_ascii {
        render_with_ascii(frame, chunks[0], state);
    } else {
        render_simple(frame, chunks[0], state);
    }

    // Render help bar
    render_help(frame, chunks[1]);
}

fn render_with_ascii(frame: &mut Frame, area: Rect, state: &LandingState) {
    // For large terminals, show the full ASCII art with text beside it
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),         // Main content area
            Constraint::Length(4),      // Mode selection
        ])
        .split(area);
    
    // Split horizontally for ASCII and content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(122),    // ASCII art width + padding
            Constraint::Min(0),         // Text content
        ])
        .split(main_chunks[0]);
    
    // Center the ASCII art vertically
    let ascii_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((content_chunks[0].height.saturating_sub(44)) / 2), // Top padding
            Constraint::Length(44),     // ASCII art height
            Constraint::Min(0),         // Bottom padding
        ])
        .split(content_chunks[0]);
    
    // Render ASCII art
    let ascii_lines: Vec<Line> = CUPCAKE_ASCII
        .lines()
        .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::Magenta))))
        .collect();
    
    let ascii = Paragraph::new(ascii_lines)
        .alignment(Alignment::Left);
    
    frame.render_widget(ascii, ascii_area[1]);

    // Text content beside ASCII
    let text_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((content_chunks[1].height.saturating_sub(8)) / 2), // Center vertically
            Constraint::Length(2),      // Title
            Constraint::Length(1),      // Space
            Constraint::Length(3),      // Description
            Constraint::Min(0),         // Rest
        ])
        .split(content_chunks[1]);
    
    // Render title
    let title_lines = vec![
        Line::from(vec![
            Span::styled("CUPCAKE", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("Policy Enforcement for AI Coding Agents"),
        ]),
    ];
    
    let title = Paragraph::new(title_lines)
        .alignment(Alignment::Left);
    
    frame.render_widget(title, text_area[1]);

    // Render description
    let description = vec![
        Line::from(vec![
            Span::raw("Turn your "),
            Span::styled("rules", Style::default().fg(Color::Yellow)),
            Span::raw(" into enforced policies and performance improvements."),
        ]),
        Line::from(vec![
            Span::raw("Cupcake will auto-create policies and hooks from your existing rules."),
        ]),
        Line::from(vec![
            Span::styled("You decide which hooks to keep", Style::default().fg(Color::Green)),
            Span::raw("."),
        ]),
    ];

    let desc_widget = Paragraph::new(description)
        .alignment(Alignment::Left);
    
    frame.render_widget(desc_widget, text_area[3]);
    
    // Mode selection at bottom
    render_mode_selection(frame, main_chunks[1], state);
}

fn render_simple(frame: &mut Frame, area: Rect, state: &LandingState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12),     // ASCII + title area
            Constraint::Length(1),      // Space
            Constraint::Min(0),         // Description
        ])
        .split(area);

    // Split the top area horizontally for ASCII and title
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),     // ASCII art
            Constraint::Min(0),         // Title
        ])
        .split(chunks[0]);

    // Simple ASCII art
    let simple_ascii = r#"      ,
            |`-.__
            / ' _/
           ****` 
          /    }
         /  \ /
     \ /`   \\
      `\    /_\
       `~~~~~``~`"#;
    
    let ascii_lines: Vec<Line> = simple_ascii
        .lines()
        .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::Magenta))))
        .collect();
    
    let ascii = Paragraph::new(ascii_lines)
        .alignment(Alignment::Left);
    
    frame.render_widget(ascii, top_chunks[0]);

    // Title beside ASCII
    let title_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Padding
            Constraint::Length(2),      // Title text
            Constraint::Min(0),         // Rest
        ])
        .split(top_chunks[1]);
    
    let title_lines = vec![
        Line::from(vec![
            Span::styled("CUPCAKE", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("Policy Enforcement for AI Coding Agents"),
    ];
    
    let title = Paragraph::new(title_lines)
        .alignment(Alignment::Left);
    
    frame.render_widget(title, title_area[1]);

    // Simple description
    render_simple_description(frame, chunks[2], state);
}

fn render_description(frame: &mut Frame, area: Rect) {
    let description = vec![
        Line::from(vec![
            Span::raw("Turn your "),
            Span::styled("rules", Style::default().fg(Color::Yellow)),
            Span::raw(" into enforced policies. "),
            Span::styled("You decide which hooks to keep.", Style::default().fg(Color::Green)),
        ]),
    ];

    let desc_widget = Paragraph::new(description)
        .alignment(Alignment::Center);
    
    frame.render_widget(desc_widget, area);
}

fn render_simple_description(frame: &mut Frame, area: Rect, state: &LandingState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Simple message
            Constraint::Length(1),      // Space
            Constraint::Min(0),         // Mode selection
        ])
        .split(area);
    
    // Simple message
    let description = vec![
        Line::from(vec![
            Span::raw("Turn your "),
            Span::styled("rules", Style::default().fg(Color::Yellow)),
            Span::raw(" into enforced policies and performance improvements."),
        ]),
        Line::from(vec![
            Span::raw("Cupcake will auto-create policies and hooks from your existing rules."),
        ]),
        Line::from(vec![
            Span::styled("You decide which hooks to keep", Style::default().fg(Color::Green)),
            Span::raw(". You can also use an intelligent rules/hook builder from scratch."),
        ]),
    ];

    let desc_widget = Paragraph::new(description)
        .alignment(Alignment::Center);
    
    frame.render_widget(desc_widget, chunks[0]);
    
    // Mode selection with clear visual
    render_mode_selection(frame, chunks[2], state);
}


fn render_mode_selection(frame: &mut Frame, area: Rect, state: &LandingState) {
    let mode_text = if state.auto_discovery {
        vec![
            Line::from(vec![
                Span::styled("▶ Auto-discover", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled("existing rule files", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::raw("  Manual create"),
                Span::raw("  "),
                Span::styled("write rules from scratch", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("↑↓", Style::default().fg(Color::Cyan)),
                Span::raw(" to switch modes"),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::raw("  Auto-discover"),
                Span::raw("  "),
                Span::styled("existing rule files", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("▶ Manual create", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled("write rules from scratch", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("↑↓", Style::default().fg(Color::Cyan)),
                Span::raw(" to switch modes"),
            ]),
        ]
    };

    let mode_widget = Paragraph::new(mode_text)
        .alignment(Alignment::Center)
        .block(Block::default()
            .borders(Borders::NONE));
    
    frame.render_widget(mode_widget, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::raw(" Press "),
        Span::styled("Space", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" to begin"),
    ]);
    
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::DarkGray));
    
    frame.render_widget(help, area);
}