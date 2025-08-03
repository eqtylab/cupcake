use crate::engine::events::{AgentEvent, HookEvent};
use crate::{CupcakeError, Result};
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

        if self.debug {
            eprintln!("Debug: Raw stdin input: {}", input.trim());
        }

        // First try to parse as ClaudeCodeEvent (currently our only agent type)
        let claude_event: HookEvent = serde_json::from_str(&input)
            .map_err(|e| CupcakeError::HookEvent(format!("Invalid JSON from stdin: {e}")))?;
        
        // Wrap in AgentEvent
        Ok(AgentEvent::ClaudeCode(claude_event))
    }

    fn log_stdin_content(&self, content: &str) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/cupcake-debug.log")
        {
            use std::io::Write;
            let _ = writeln!(
                file,
                "  STDIN received: {}",
                if content.trim().is_empty() {
                    "[EMPTY]"
                } else {
                    content.trim()
                }
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
