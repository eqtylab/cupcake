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
    // Center everything vertically
    let total_height = 30; // Approximate height of all content
    let vertical_padding = area.height.saturating_sub(total_height) / 2;
    
    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(vertical_padding),
            Constraint::Min(0),
            Constraint::Length(vertical_padding),
        ])
        .split(area);
    
    let content_area = outer_chunks[1];
    
    // Create centered content with proper ASCII art
    let mut lines = Vec::new();
    
    // ASCII art block (will be left-aligned within centered container)
    let ascii_lines = vec![
        "            ,",
        "            |`-.__",
        "            / ' _/",
        "           ****` ",
        "          /    }",
        "         /  \\ /",
        "     \\ /`   \\\\\\",
        "      `\\    /_\\\\",
        "       `~~~~~``~`",
    ];
    
    // Add ASCII art
    for ascii_line in ascii_lines {
        lines.push(Line::from(Span::styled(ascii_line, Style::default().fg(Color::Cyan))));
    }
    
    // Spacing after ASCII
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    
    // Title and subtitle (will be center-aligned)
    lines.push(Line::from(Span::styled("CUPCAKE", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))));
    lines.push(Line::from("Policy Enforcement for AI Coding Agents"));
    
    // Add spacing
    lines.push(Line::from(""));
    
    // Description
    lines.push(Line::from("Turn your rules into enforced policies and performance improvements."));
    lines.push(Line::from("Cupcake will auto-create policies and hooks from your existing rules."));
    lines.push(Line::from(vec![
        Span::styled("You decide which hooks to keep", Style::default().fg(Color::Green)),
        Span::raw(". "),
        Span::styled("You can also use an intelligent rules/hook builder from scratch", Style::default().fg(Color::Yellow)),
        Span::raw("."),
    ]));
    
    // Add spacing
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    
    // Mode selection
    let auto_icon = if state.auto_discovery { "▶" } else { "  " };
    let manual_icon = if !state.auto_discovery { "▶" } else { "  " };
    
    lines.push(Line::from(vec![
        Span::raw(auto_icon),
        Span::raw(" "),
        Span::styled("Auto-discover", if state.auto_discovery {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        }),
        Span::raw("  existing rule files"),
    ]));
    
    lines.push(Line::from(vec![
        Span::raw(manual_icon),
        Span::raw(" "),
        Span::styled("Manual create", if !state.auto_discovery {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        }),
        Span::raw("  write rules from scratch"),
    ]));
    
    lines.push(Line::from(""));
    lines.push(Line::from("↑↓ to switch modes"));
    
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    
    lines.push(Line::from(Span::styled("Press Enter to begin", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
    
    // Split the content into ASCII art section and text section
    let ascii_end = 11; // 9 ASCII lines + 2 spacing lines
    let ascii_lines = lines[0..ascii_end].to_vec();
    let text_lines = lines[ascii_end..].to_vec();
    
    // Create layout for ASCII art (centered container) and text (center-aligned)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(ascii_end as u16), // ASCII art section
            Constraint::Min(0),                    // Text section
        ])
        .split(content_area);
    
    // Render ASCII art in a centered container with left alignment
    let ascii_width = 20; // Width of ASCII art
    let ascii_padding = chunks[0].width.saturating_sub(ascii_width) / 2;
    let ascii_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(ascii_padding),
            Constraint::Length(ascii_width),
            Constraint::Length(ascii_padding),
        ])
        .split(chunks[0])[1];
    
    let ascii_paragraph = Paragraph::new(ascii_lines)
        .alignment(Alignment::Left);
    frame.render_widget(ascii_paragraph, ascii_area);
    
    // Render text content with center alignment
    let text_paragraph = Paragraph::new(text_lines)
        .alignment(Alignment::Center);
    frame.render_widget(text_paragraph, chunks[1]);
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
        Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" to begin"),
    ]);
    
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::DarkGray));
    
    frame.render_widget(help, area);
}