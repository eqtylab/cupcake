use crate::engine::events::{AgentEvent, ClaudeCodeEvent};
use crate::{CupcakeError, Result, tracing::trace};
use std::io::Read;

/// Parses hook events from stdin
pub struct HookEventParser {
    debug: bool,
}

impl HookEventParser {
    pub fn new(debug: bool) -> Self {
        Self { debug }
    }

    /// Parse hook event JSON from stdin
    pub fn parse_from_stdin(&self) -> Result<AgentEvent> {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;

        self.log_stdin_content(&input);

        if input.trim().is_empty() {
            return Err(CupcakeError::HookEvent(
                "No input received from stdin".to_string(),
            ));
        }

        trace!(stdin_input = %input.trim(), "Raw stdin input");

        // First try to parse as ClaudeCodeEvent (currently our only agent type)
        let claude_event: ClaudeCodeEvent = serde_json::from_str(&input)
            .map_err(|e| CupcakeError::HookEvent(format!("Invalid JSON from stdin: {e}")))?;

        // Wrap in AgentEvent
        Ok(AgentEvent::ClaudeCode(claude_event))
    }

    fn log_stdin_content(&self, content: &str) {
        let stdin_content = if content.trim().is_empty() {
            "[EMPTY]"
        } else {
            content.trim()
        };
        
        trace!(stdin_content = %stdin_content, "STDIN received");
        
        // Keep file logging for backward compatibility
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/cupcake-debug.log")
        {
            use std::io::Write;
            let _ = writeln!(
                file,
                "  STDIN received: {}",
                stdin_content
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = HookEventParser::new(true);
        assert!(parser.debug);

        let parser = HookEventParser::new(false);
        assert!(!parser.debug);
    }
}
