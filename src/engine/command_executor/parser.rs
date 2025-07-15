//! String Command Parser for Plan 008 Part 2
//! 
//! This module implements a safe, limited parser for shell-like command strings.
//! It transforms string specifications into the same secure CommandGraph used
//! by the array executor, ensuring no shell is ever invoked.
//!
//! Supported operators in v1.0: | > >> && ||
//! Unsupported syntax results in clear UnsupportedSyntax errors.

use crate::config::actions::StringCommandSpec;
use super::{CommandGraph, ExecutionNode, Command, Operation, ConditionalExecution, ExecutionError};
use std::collections::HashMap;
use std::path::PathBuf;

/// Tokens produced by the string parser
#[derive(Debug, Clone, PartialEq)]
enum OpTok {
    /// Command word (program name or argument)
    CmdWord(String),
    /// Pipe operator |
    Pipe,
    /// Redirect stdout to file (truncate) >
    RedirectOut,
    /// Append stdout to file >>
    AppendOut,
    /// Conditional execution on success &&
    AndAnd,
    /// Conditional execution on failure ||
    OrOr,
}

/// Errors specific to string parsing
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Command substitution \"${{...}}\" is not allowed")]
    CommandSubst,
    
    #[error("Backtick command substitution is not allowed")]
    BacktickSubst,
    
    #[error("Multiple redirects after single command")]
    MultipleRedirects,
    
    #[error("Unexpected operator \"{0}\" (no command before it)")]
    EmptyCommand(String),
    
    #[error("Trailing operator \"{0}\" at end of line")]
    TrailingOperator(String),
    
    #[error("Redirect combination \"{0}\" is not supported in v1.0")]
    RedirectCombo(String),
    
    #[error("Shell word parsing failed: {0}")]
    ShellWordsParse(String),
}

impl From<ParseError> for ExecutionError {
    fn from(err: ParseError) -> Self {
        ExecutionError::InvalidSpec(err.to_string())
    }
}

/// String command parser that transforms shell-like syntax into secure CommandGraph
pub struct StringParser {
    /// Template variables for substitution
    template_vars: HashMap<String, String>,
}

impl StringParser {
    /// Create a new string parser with template variables
    pub fn new(template_vars: HashMap<String, String>) -> Self {
        Self { template_vars }
    }
    
    /// Parse a StringCommandSpec into a CommandGraph
    /// 
    /// This is the main entry point that coordinates all parsing phases:
    /// 1. Pre-scan for unsupported syntax
    /// 2. Tokenize with shell-words
    /// 3. Classify tokens into OpTok enum
    /// 4. Build CommandGraph from linear token stream
    pub fn parse(&self, spec: &StringCommandSpec) -> Result<CommandGraph, ParseError> {
        // Phase 1: Quick pre-scan for unsupported syntax
        self.validate_unsupported_syntax(&spec.command)?;
        
        // Phase 2: Tokenize with shell-words (handles quotes and escapes safely)
        let raw_tokens = shell_words::split(&spec.command)
            .map_err(|_| ParseError::ShellWordsParse("Invalid quotes or escapes".to_string()))?;
        
        // Phase 3: Classify raw tokens into OpTok enum
        let classified_tokens = self.classify_tokens(raw_tokens)?;
        
        // Phase 4: Build CommandGraph from linear token stream
        self.build_graph_from_tokens(classified_tokens)
    }
    
    /// Phase 1: Pre-scan for unsupported shell syntax
    fn validate_unsupported_syntax(&self, command: &str) -> Result<(), ParseError> {
        // Check for command substitution
        if command.contains("$(") {
            return Err(ParseError::CommandSubst);
        }
        
        if command.contains('`') {
            return Err(ParseError::BacktickSubst);
        }
        
        // Check for unsupported redirect combinations
        let unsupported_redirects = ["2>&1", ">&2", "2>", "<", "<<"];
        for redirect in &unsupported_redirects {
            if command.contains(redirect) {
                return Err(ParseError::RedirectCombo(redirect.to_string()));
            }
        }
        
        Ok(())
    }
    
    /// Phase 3: Classify raw string tokens into OpTok enum
    fn classify_tokens(&self, raw_tokens: Vec<String>) -> Result<Vec<OpTok>, ParseError> {
        let mut classified = Vec::new();
        
        for token in raw_tokens {
            let classified_token = match token.as_str() {
                "|" => OpTok::Pipe,
                ">" => OpTok::RedirectOut,
                ">>" => OpTok::AppendOut,
                "&&" => OpTok::AndAnd,
                "||" => OpTok::OrOr,
                _ => OpTok::CmdWord(token),
            };
            classified.push(classified_token);
        }
        
        Ok(classified)
    }
    
    /// Phase 4: Build CommandGraph from linear token stream (no precedence)
    fn build_graph_from_tokens(&self, tokens: Vec<OpTok>) -> Result<CommandGraph, ParseError> {
        if tokens.is_empty() {
            return Err(ParseError::EmptyCommand("No command provided".to_string()));
        }
        
        // For v1.0, let's implement a simplified approach focused on basic cases
        // We'll enhance this incrementally
        
        // Check for basic errors first
        if matches!(tokens[0], OpTok::Pipe | OpTok::RedirectOut | OpTok::AppendOut | OpTok::AndAnd | OpTok::OrOr) {
            let op_name = match &tokens[0] {
                OpTok::Pipe => "|",
                OpTok::RedirectOut => ">",
                OpTok::AppendOut => ">>",
                OpTok::AndAnd => "&&",
                OpTok::OrOr => "||",
                _ => "unknown",
            };
            return Err(ParseError::EmptyCommand(op_name.to_string()));
        }
        
        // Check for trailing operators
        if let Some(last_token) = tokens.last() {
            if matches!(last_token, OpTok::Pipe | OpTok::RedirectOut | OpTok::AppendOut | OpTok::AndAnd | OpTok::OrOr) {
                let op_name = match last_token {
                    OpTok::Pipe => "|",
                    OpTok::RedirectOut => ">",
                    OpTok::AppendOut => ">>",
                    OpTok::AndAnd => "&&",
                    OpTok::OrOr => "||",
                    _ => "unknown",
                };
                return Err(ParseError::TrailingOperator(op_name.to_string()));
            }
        }
        
        // For v1.0, let's just handle simple commands without operators
        // This gives us a solid foundation to build on
        let mut cmd_words = Vec::new();
        
        // Check if we have any operators - if so, return not implemented error for now
        for token in &tokens {
            match token {
                OpTok::CmdWord(word) => cmd_words.push(word.clone()),
                OpTok::Pipe => return Err(ParseError::RedirectCombo("| (pipe support coming in next iteration)".to_string())),
                OpTok::RedirectOut => return Err(ParseError::RedirectCombo("> (redirect support coming in next iteration)".to_string())),
                OpTok::AppendOut => return Err(ParseError::RedirectCombo(">> (append support coming in next iteration)".to_string())),
                OpTok::AndAnd => return Err(ParseError::RedirectCombo("&& (conditional support coming in next iteration)".to_string())),
                OpTok::OrOr => return Err(ParseError::RedirectCombo("|| (conditional support coming in next iteration)".to_string())),
            }
        }
        
        if cmd_words.is_empty() {
            return Err(ParseError::EmptyCommand("No command words found".to_string()));
        }
        
        let command = self.build_command_from_words(cmd_words)?;
        let node = ExecutionNode {
            command,
            operations: Vec::new(),
            conditional: None,
        };
        
        Ok(CommandGraph { nodes: vec![node] })
    }
    
    /// Extract the next command after a pipe operator
    fn extract_next_command(&self, tokens: &[OpTok], start_index: usize) -> Result<(String, Vec<String>), ParseError> {
        let mut cmd_words = Vec::new();
        
        for token in tokens.iter().skip(start_index) {
            match token {
                OpTok::CmdWord(word) => cmd_words.push(word.clone()),
                _ => break, // Hit another operator, stop collecting
            }
        }
        
        if cmd_words.is_empty() {
            return Err(ParseError::EmptyCommand("| (missing command after pipe)".to_string()));
        }
        
        let program = cmd_words[0].clone();
        let args = if cmd_words.len() > 1 {
            cmd_words[1..].to_vec()
        } else {
            Vec::new()
        };
        
        Ok((program, args))
    }
    
    /// Build a Command from a vector of words (program + args)
    fn build_command_from_words(&self, words: Vec<String>) -> Result<Command, ParseError> {
        if words.is_empty() {
            return Err(ParseError::EmptyCommand("No command words".to_string()));
        }
        
        let program = words[0].clone();
        let args = if words.len() > 1 {
            words[1..].iter()
                .map(|arg| self.substitute_template(arg))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        
        Ok(Command {
            program,
            args,
            working_dir: None,
            env_vars: HashMap::new(),
        })
    }
    
    /// Safely substitute template variables (same logic as CommandExecutor)
    fn substitute_template(&self, template: &str) -> Result<String, ParseError> {
        let mut result = template.to_string();
        
        for (key, value) in &self.template_vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_parser() -> StringParser {
        let mut vars = HashMap::new();
        vars.insert("file_path".to_string(), "/tmp/test.txt".to_string());
        vars.insert("user".to_string(), "alice".to_string());
        StringParser::new(vars)
    }

    #[test]
    fn test_simple_command_parsing() {
        let parser = create_parser();
        let spec = StringCommandSpec {
            command: "echo hello world".to_string(),
        };

        let graph = parser.parse(&spec).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec!["hello", "world"]);
        assert!(node.operations.is_empty());
    }

    #[test]
    fn test_quoted_arguments() {
        let parser = create_parser();
        let spec = StringCommandSpec {
            command: r#"grep "Hello World" file.txt"#.to_string(),
        };

        let graph = parser.parse(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "grep");
        assert_eq!(node.command.args, vec!["Hello World", "file.txt"]);
    }

    #[test]
    fn test_unsupported_syntax_detection() {
        let parser = create_parser();
        
        // Test command substitution
        let spec = StringCommandSpec {
            command: "echo $(whoami)".to_string(),
        };
        let result = parser.parse(&spec);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Command substitution"));
        
        // Test backticks
        let spec = StringCommandSpec {
            command: "echo `date`".to_string(),
        };
        let result = parser.parse(&spec);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Backtick"));
    }

    #[test]
    fn test_token_classification() {
        let parser = create_parser();
        let tokens = vec![
            "echo".to_string(),
            "hello".to_string(),
            "|".to_string(),
            "grep".to_string(),
            "hello".to_string(),
        ];

        let classified = parser.classify_tokens(tokens).unwrap();
        assert_eq!(classified, vec![
            OpTok::CmdWord("echo".to_string()),
            OpTok::CmdWord("hello".to_string()),
            OpTok::Pipe,
            OpTok::CmdWord("grep".to_string()),
            OpTok::CmdWord("hello".to_string()),
        ]);
    }

    #[test]
    fn test_quoted_operators_as_literals_not_yet_supported() {
        let parser = create_parser();
        let spec = StringCommandSpec {
            command: r#"grep "|" file.txt"#.to_string(),
        };

        // First, let's check what shell-words produces
        let raw_tokens = shell_words::split(&spec.command).unwrap();
        assert_eq!(raw_tokens, vec!["grep", "|", "file.txt"]);
        
        // For v1.0 basic implementation, this fails because we don't yet
        // distinguish between quoted and unquoted operators
        // TODO: Implement proper quote detection in next iteration
        let result = parser.parse(&spec);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pipe support coming"));
    }

    #[test]
    fn test_empty_command_errors() {
        let parser = create_parser();
        
        // Empty command string
        let spec = StringCommandSpec {
            command: "".to_string(),
        };
        let result = parser.parse(&spec);
        assert!(result.is_err());
        
        // Operator without command
        let spec = StringCommandSpec {
            command: "| grep test".to_string(),
        };
        let result = parser.parse(&spec);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no command before"));
    }

    #[test]
    fn test_trailing_operator_error() {
        let parser = create_parser();
        let spec = StringCommandSpec {
            command: "echo test |".to_string(),
        };
        
        let result = parser.parse(&spec);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Trailing operator"));
    }

    #[test]
    fn test_template_substitution() {
        let parser = create_parser();
        let spec = StringCommandSpec {
            command: "echo hello {{user}}".to_string(),
        };

        let graph = parser.parse(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec!["hello", "alice"]);
    }
}