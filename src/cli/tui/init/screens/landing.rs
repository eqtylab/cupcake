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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),      // Top padding
            Constraint::Length(44),     // ASCII art
            Constraint::Length(1),      // Space
            Constraint::Length(2),      // Title
            Constraint::Length(1),      // Space
            Constraint::Length(1),      // Description
            Constraint::Length(1),      // Space
            Constraint::Min(0),         // Mode selection
        ])
        .split(area);

    // Render ASCII art
    let ascii_lines: Vec<Line> = CUPCAKE_ASCII
        .lines()
        .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::Magenta))))
        .collect();
    
    let ascii = Paragraph::new(ascii_lines)
        .alignment(Alignment::Center);
    
    frame.render_widget(ascii, chunks[1]);

    // Render title
    let title_lines = vec![
        Line::from(vec![
            Span::styled("CUPCAKE", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Policy Enforcement for AI Coding Agents"),
        ]),
    ];
    
    let title = Paragraph::new(title_lines)
        .alignment(Alignment::Center);
    
    frame.render_widget(title, chunks[3]);

    // Render description
    render_description(frame, chunks[5]);
    
    // Mode selection
    render_mode_selection(frame, chunks[7], state);
}

fn render_simple(frame: &mut Frame, area: Rect, state: &LandingState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),     // ASCII logo
            Constraint::Length(1),      // Space
            Constraint::Length(3),      // Title & subtitle
            Constraint::Length(1),      // Space
            Constraint::Min(0),         // Simple description
        ])
        .split(area);

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
        .alignment(Alignment::Center);
    
    frame.render_widget(ascii, chunks[0]);

    // Title
    let title_lines = vec![
        Line::from(vec![
            Span::styled("CUPCAKE", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("Policy Enforcement for AI Coding Agents"),
    ];
    
    let title = Paragraph::new(title_lines)
        .alignment(Alignment::Center);
    
    frame.render_widget(title, chunks[2]);

    // Simple description
    render_simple_description(frame, chunks[4], state);
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
                Span::styled("Tab", Style::default().fg(Color::Cyan)),
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
                Span::styled("Tab", Style::default().fg(Color::Cyan)),
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