//! Landing screen with ASCII art and introduction

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
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
            Constraint::Length(1),      // Top padding
            Constraint::Min(0),         // Main content
            Constraint::Length(3),      // Options
            Constraint::Length(1),      // Help bar
        ])
        .split(frame.area());

    // Check if terminal is large enough for ASCII art
    let ascii_width = 120;
    let ascii_height = 43;
    let show_ascii = frame.area().width >= ascii_width && frame.area().height >= (ascii_height + 10);

    if show_ascii {
        render_with_ascii(frame, chunks[1], state);
    } else {
        render_simple(frame, chunks[1], state);
    }

    // Render options
    render_options(frame, chunks[2], state);

    // Render help bar
    render_help(frame, chunks[3]);
}

fn render_with_ascii(frame: &mut Frame, area: Rect, state: &LandingState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(44),     // ASCII art
            Constraint::Length(1),      // Space
            Constraint::Length(3),      // Title
            Constraint::Length(1),      // Space
            Constraint::Min(0),         // Description
        ])
        .split(area);

    // Render ASCII art
    let ascii_lines: Vec<Line> = CUPCAKE_ASCII
        .lines()
        .map(|line| Line::from(Span::raw(line)))
        .collect();
    
    let ascii = Paragraph::new(ascii_lines)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Magenta));
    
    frame.render_widget(ascii, chunks[0]);

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
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));
    
    frame.render_widget(title, chunks[2]);

    // Render description
    render_description(frame, chunks[4]);
}

fn render_simple(frame: &mut Frame, area: Rect, _state: &LandingState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),      // Simple logo
            Constraint::Length(1),      // Space
            Constraint::Min(0),         // Description
        ])
        .split(area);

    // Simple text logo for small terminals
    let logo_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("🧁 CUPCAKE", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from("Policy Enforcement for AI Coding Agents"),
    ];
    
    let logo = Paragraph::new(logo_lines)
        .alignment(Alignment::Center);
    
    frame.render_widget(logo, chunks[0]);

    // Render description
    render_description(frame, chunks[2]);
}

fn render_description(frame: &mut Frame, area: Rect) {
    let description = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Welcome to Cupcake Init Wizard", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from("This wizard will help you:"),
        Line::from(""),
        Line::from("  1. Discover existing AI agent rule files (CLAUDE.md, .cursorrules, etc.)"),
        Line::from("  2. Extract security and coding rules from your documentation"),
        Line::from("  3. Convert natural language rules into enforceable policies"),
        Line::from("  4. Configure hooks for your AI coding environment"),
        Line::from(""),
        Line::from("Cupcake ensures your AI agents follow your team's standards by:"),
        Line::from("  • Blocking dangerous operations before they execute"),
        Line::from("  • Providing warnings for questionable practices"),
        Line::from("  • Enforcing consistent coding patterns"),
        Line::from("  • Creating an audit trail of AI actions"),
    ];

    let desc_widget = Paragraph::new(description)
        .block(Block::default()
            .borders(Borders::NONE))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    
    frame.render_widget(desc_widget, area);
}

fn render_options(frame: &mut Frame, area: Rect, state: &LandingState) {
    let options = if state.auto_discovery {
        vec![
            Line::from(vec![
                Span::raw("  "),
                Span::styled("[✓]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" Auto-discovery mode  "),
                Span::styled("[ ]", Style::default().fg(Color::DarkGray)),
                Span::raw(" Manual rule creation"),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::raw("  "),
                Span::styled("[ ]", Style::default().fg(Color::DarkGray)),
                Span::raw(" Auto-discovery mode  "),
                Span::styled("[✓]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" Manual rule creation"),
            ]),
        ]
    };

    let options_widget = Paragraph::new(options)
        .alignment(Alignment::Center);
    
    frame.render_widget(options_widget, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::raw(" "),
        Span::styled("Space", Style::default().fg(Color::Cyan)),
        Span::raw(" Start  "),
        Span::styled("•", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("Tab", Style::default().fg(Color::Cyan)),
        Span::raw(" Switch mode  "),
        Span::styled("•", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("Esc or Q", Style::default().fg(Color::Cyan)),
        Span::raw(" Exit"),
    ]);
    
    let help = Paragraph::new(help_text)
        .style(Style::default().bg(Color::DarkGray));
    
    frame.render_widget(help, area);
}