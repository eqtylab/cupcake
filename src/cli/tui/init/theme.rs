use ratatui::style::{Color, Modifier, Style};

/// Consistent theme for the TUI
pub struct Theme {
    pub selected: Style,
    pub focused: Style,
    pub error: Style,
    pub success: Style,
    pub critical: Style,
    pub warning: Style,
    pub info: Style,
    pub muted: Style,
    pub highlight: Style,
    pub modal_border: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            selected: Style::default()
                .bg(Color::Rgb(50, 50, 80))
                .add_modifier(Modifier::BOLD),
            focused: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            error: Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
            success: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            critical: Style::default()
                .fg(Color::Red),
            warning: Style::default()
                .fg(Color::Yellow),
            info: Style::default()
                .fg(Color::Blue),
            muted: Style::default()
                .fg(Color::DarkGray),
            highlight: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            modal_border: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        }
    }
}

impl Theme {
    /// Get color for agent badges
    pub fn agent_color(&self, agent: &crate::cli::tui::init::state::Agent) -> Color {
        use crate::cli::tui::init::state::Agent;
        match agent {
            Agent::Claude => Color::Cyan,
            Agent::Cursor => Color::Magenta,
            Agent::Windsurf => Color::Blue,
            Agent::Kiro => Color::Green,
            Agent::Copilot => Color::Yellow,
            Agent::Aider => Color::Red,
            Agent::Gemini => Color::LightBlue,
        }
    }

    /// Get emoji for severity
    pub fn severity_emoji(severity: &crate::cli::tui::init::state::Severity) -> &'static str {
        use crate::cli::tui::init::state::Severity;
        match severity {
            Severity::High => "🔴",
            Severity::Medium => "🟡",
            Severity::Low => "🔵",
        }
    }

    /// Get style for severity
    pub fn severity_style(&self, severity: &crate::cli::tui::init::state::Severity) -> Style {
        use crate::cli::tui::init::state::Severity;
        match severity {
            Severity::High => self.critical,
            Severity::Medium => self.warning,
            Severity::Low => self.info,
        }
    }

    /// Get status icon
    pub fn status_icon(status: &crate::cli::tui::init::state::TaskStatus) -> &'static str {
        use crate::cli::tui::init::state::TaskStatus;
        match status {
            TaskStatus::Queued => "⏳",
            TaskStatus::InProgress => "⟳",
            TaskStatus::Complete => "✓",
            TaskStatus::Failed(_) => "✗",
        }
    }

    /// Get phase status icon
    pub fn phase_icon(status: &crate::cli::tui::init::state::PhaseStatus) -> &'static str {
        use crate::cli::tui::init::state::PhaseStatus;
        match status {
            PhaseStatus::Pending => "⏳",
            PhaseStatus::InProgress => "⟳",
            PhaseStatus::Complete => "✓",
            PhaseStatus::Failed(_) => "✗",
        }
    }
}