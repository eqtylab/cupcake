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
        
        // V1.0 implementation: Linear parsing of operators
        let mut nodes = Vec::new();
        let mut current_words = Vec::new();
        let mut i = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                OpTok::CmdWord(word) => {
                    current_words.push(word.clone());
                    i += 1;
                }
                OpTok::Pipe => {
                    // Build current command and prepare for pipe
                    if current_words.is_empty() {
                        return Err(ParseError::EmptyCommand("| (no command before pipe)".to_string()));
                    }
                    
                    let command = self.build_command_from_words(current_words)?;
                    current_words = Vec::new();
                    i += 1;
                    
                    // Collect piped commands
                    let mut operations = Vec::new();
                    while i < tokens.len() {
                        if matches!(tokens[i], OpTok::Pipe | OpTok::AndAnd | OpTok::OrOr) {
                            break;
                        }
                        
                        // Collect command words until next operator
                        let mut pipe_words = Vec::new();
                        while i < tokens.len() && matches!(tokens[i], OpTok::CmdWord(_)) {
                            if let OpTok::CmdWord(word) = &tokens[i] {
                                pipe_words.push(word.clone());
                            }
                            i += 1;
                        }
                        
                        if pipe_words.is_empty() {
                            return Err(ParseError::EmptyCommand("| (missing command after pipe)".to_string()));
                        }
                        
                        let pipe_cmd = self.build_pipe_command(pipe_words)?;
                        operations.push(Operation::Pipe(pipe_cmd));
                        
                        // Check for redirect after pipe
                        if i < tokens.len() {
                            match &tokens[i] {
                                OpTok::RedirectOut => {
                                    i += 1;
                                    if i >= tokens.len() || !matches!(tokens[i], OpTok::CmdWord(_)) {
                                        return Err(ParseError::TrailingOperator(">".to_string()));
                                    }
                                    if let OpTok::CmdWord(file) = &tokens[i] {
                                        operations.push(Operation::RedirectStdout(file.into()));
                                        i += 1;
                                    }
                                }
                                OpTok::AppendOut => {
                                    i += 1;
                                    if i >= tokens.len() || !matches!(tokens[i], OpTok::CmdWord(_)) {
                                        return Err(ParseError::TrailingOperator(">>".to_string()));
                                    }
                                    if let OpTok::CmdWord(file) = &tokens[i] {
                                        operations.push(Operation::AppendStdout(file.into()));
                                        i += 1;
                                    }
                                }
                                OpTok::Pipe => {
                                    i += 1;
                                    continue; // Continue processing pipe chain
                                }
                                _ => break, // Exit pipe processing for other operators
                            }
                        }
                    }
                    
                    nodes.push(ExecutionNode {
                        command,
                        operations,
                        conditional: None,
                    });
                }
                OpTok::RedirectOut => {
                    // Handle redirect without pipe
                    if current_words.is_empty() {
                        return Err(ParseError::EmptyCommand("> (no command before redirect)".to_string()));
                    }
                    
                    i += 1;
                    if i >= tokens.len() || !matches!(tokens[i], OpTok::CmdWord(_)) {
                        return Err(ParseError::TrailingOperator(">".to_string()));
                    }
                    
                    let command = self.build_command_from_words(current_words)?;
                    current_words = Vec::new();
                    
                    let mut operations = vec![];
                    if let OpTok::CmdWord(file) = &tokens[i] {
                        operations.push(Operation::RedirectStdout(file.into()));
                        i += 1;
                    }
                    
                    nodes.push(ExecutionNode {
                        command,
                        operations,
                        conditional: None,
                    });
                }
                OpTok::AppendOut => {
                    // Handle append without pipe
                    if current_words.is_empty() {
                        return Err(ParseError::EmptyCommand(">> (no command before append)".to_string()));
                    }
                    
                    i += 1;
                    if i >= tokens.len() || !matches!(tokens[i], OpTok::CmdWord(_)) {
                        return Err(ParseError::TrailingOperator(">>".to_string()));
                    }
                    
                    let command = self.build_command_from_words(current_words)?;
                    current_words = Vec::new();
                    
                    let mut operations = vec![];
                    if let OpTok::CmdWord(file) = &tokens[i] {
                        operations.push(Operation::AppendStdout(file.into()));
                        i += 1;
                    }
                    
                    nodes.push(ExecutionNode {
                        command,
                        operations,
                        conditional: None,
                    });
                }
                OpTok::AndAnd | OpTok::OrOr => {
                    // Handle conditional execution
                    if current_words.is_empty() {
                        let op = if matches!(tokens[i], OpTok::AndAnd) { "&&" } else { "||" };
                        return Err(ParseError::EmptyCommand(format!("{} (no command before conditional)", op)));
                    }
                    
                    let command = self.build_command_from_words(current_words)?;
                    current_words = Vec::new();
                    
                    let is_and = matches!(tokens[i], OpTok::AndAnd);
                    i += 1;
                    
                    // Collect the conditional command
                    let mut cond_words = Vec::new();
                    while i < tokens.len() && matches!(tokens[i], OpTok::CmdWord(_)) {
                        if let OpTok::CmdWord(word) = &tokens[i] {
                            cond_words.push(word.clone());
                        }
                        i += 1;
                    }
                    
                    if cond_words.is_empty() {
                        let op = if is_and { "&&" } else { "||" };
                        return Err(ParseError::TrailingOperator(op.to_string()));
                    }
                    
                    let cond_command = self.build_command_from_words(cond_words)?;
                    let cond_node = ExecutionNode {
                        command: cond_command,
                        operations: Vec::new(),
                        conditional: None,
                    };
                    
                    let conditional = if is_and {
                        ConditionalExecution {
                            on_success: vec![cond_node],
                            on_failure: vec![],
                        }
                    } else {
                        ConditionalExecution {
                            on_success: vec![],
                            on_failure: vec![cond_node],
                        }
                    };
                    
                    nodes.push(ExecutionNode {
                        command,
                        operations: Vec::new(),
                        conditional: Some(conditional),
                    });
                }
            }
        }
        
        // Handle any remaining words as final command
        if !current_words.is_empty() {
            let command = self.build_command_from_words(current_words)?;
            nodes.push(ExecutionNode {
                command,
                operations: Vec::new(),
                conditional: None,
            });
        }
        
        if nodes.is_empty() {
            return Err(ParseError::EmptyCommand("No commands found".to_string()));
        }
        
        Ok(CommandGraph { nodes })
    }
    
    /// Build a pipe command (simpler than full command - no env/workdir)
    fn build_pipe_command(&self, words: Vec<String>) -> Result<Command, ParseError> {
        if words.is_empty() {
            return Err(ParseError::EmptyCommand("No command words for pipe".to_string()));
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
            working_dir: None,  // Pipes inherit working dir
            env_vars: HashMap::new(),  // Pipes inherit environment
        })
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
    fn test_pipe_operator_support() {
        let parser = create_parser();
        let spec = StringCommandSpec {
            command: "echo hello | grep hello".to_string(),
        };

        let graph = parser.parse(&spec).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec!["hello"]);
        assert_eq!(node.operations.len(), 1);
        
        match &node.operations[0] {
            Operation::Pipe(cmd) => {
                assert_eq!(cmd.program, "grep");
                assert_eq!(cmd.args, vec!["hello"]);
            }
            _ => panic!("Expected Pipe operation"),
        }
    }

    #[test]
    fn test_redirect_operator_support() {
        let parser = create_parser();
        let spec = StringCommandSpec {
            command: "echo test > output.txt".to_string(),
        };

        let graph = parser.parse(&spec).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec!["test"]);
        assert_eq!(node.operations.len(), 1);
        
        match &node.operations[0] {
            Operation::RedirectStdout(path) => {
                assert_eq!(path.to_str().unwrap(), "output.txt");
            }
            _ => panic!("Expected RedirectStdout operation"),
        }
    }

    #[test]
    fn test_append_operator_support() {
        let parser = create_parser();
        let spec = StringCommandSpec {
            command: "echo test >> output.txt".to_string(),
        };

        let graph = parser.parse(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.operations.len(), 1);
        
        match &node.operations[0] {
            Operation::AppendStdout(path) => {
                assert_eq!(path.to_str().unwrap(), "output.txt");
            }
            _ => panic!("Expected AppendStdout operation"),
        }
    }

    #[test]
    fn test_conditional_operators_support() {
        let parser = create_parser();
        
        // Test &&
        let spec = StringCommandSpec {
            command: "test -f {{file_path}} && echo exists".to_string(),
        };
        let graph = parser.parse(&spec).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "test");
        assert_eq!(node.command.args, vec!["-f", "/tmp/test.txt"]);
        assert!(node.conditional.is_some());
        
        let cond = node.conditional.as_ref().unwrap();
        assert_eq!(cond.on_success.len(), 1);
        assert_eq!(cond.on_success[0].command.program, "echo");
        assert_eq!(cond.on_success[0].command.args, vec!["exists"]);
        assert!(cond.on_failure.is_empty());
        
        // Test ||
        let spec2 = StringCommandSpec {
            command: "test -f missing || echo not found".to_string(),
        };
        let graph2 = parser.parse(&spec2).unwrap();
        let node2 = &graph2.nodes[0];
        let cond2 = node2.conditional.as_ref().unwrap();
        assert!(cond2.on_success.is_empty());
        assert_eq!(cond2.on_failure.len(), 1);
        assert_eq!(cond2.on_failure[0].command.program, "echo");
    }

    #[test]
    fn test_complex_pipe_chain() {
        let parser = create_parser();
        let spec = StringCommandSpec {
            command: "cat {{file_path}} | grep test | wc -l > count.txt".to_string(),
        };

        let graph = parser.parse(&spec).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "cat");
        assert_eq!(node.command.args, vec!["/tmp/test.txt"]);
        assert_eq!(node.operations.len(), 3);
        
        match &node.operations[0] {
            Operation::Pipe(cmd) => {
                assert_eq!(cmd.program, "grep");
                assert_eq!(cmd.args, vec!["test"]);
            }
            _ => panic!("Expected first Pipe operation"),
        }
        
        match &node.operations[1] {
            Operation::Pipe(cmd) => {
                assert_eq!(cmd.program, "wc");
                assert_eq!(cmd.args, vec!["-l"]);
            }
            _ => panic!("Expected second Pipe operation"),
        }
        
        match &node.operations[2] {
            Operation::RedirectStdout(path) => {
                assert_eq!(path.to_str().unwrap(), "count.txt");
            }
            _ => panic!("Expected RedirectStdout operation"),
        }
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
        
        // For v1.0 implementation, quoted operators are still treated as operators
        // This is a known limitation - proper quote detection would require
        // preserving quote information from shell-words
        let result = parser.parse(&spec);
        assert!(result.is_ok());
        let _graph = result.unwrap();
        
        // The parser interprets this as "grep | file.txt" which is invalid
        // because "file.txt" is not a valid command
        // In a future version, we should detect quoted operators and treat them as literals
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