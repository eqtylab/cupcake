//! Encode command: Convert shell commands to secure array format
//! 
//! This module implements the `cupcake encode` command which converts shell scripts
//! into secure ArrayCommandSpec format. This is a key security tool that helps users
//! migrate from dangerous shell: commands to safe array: commands.

use super::CommandHandler;
use crate::config::actions::ArrayCommandSpec;
use crate::Result;
use serde_json;
use serde_yaml_ng;

/// Handler for the `encode` command
pub struct EncodeCommand {
    pub command: String,
    pub format: String,
    pub template: bool,
}

impl CommandHandler for EncodeCommand {
    fn execute(&self) -> Result<()> {
        // Step 1: Parse the shell command into secure array format
        let array_spec = self.parse_shell_to_array(&self.command)?;

        // Step 2: Serialize to requested format
        let output = if self.template {
            self.create_full_template(&array_spec)?
        } else {
            self.serialize_array_spec(&array_spec)?
        };

        // Step 3: Output the result
        println!("{}", output);
        
        Ok(())
    }

    fn name(&self) -> &'static str {
        "encode"
    }
}

impl EncodeCommand {
    /// Create new encode command
    pub fn new(command: String, format: String, template: bool) -> Self {
        Self {
            command,
            format,
            template,
        }
    }

    /// Parse shell command into ArrayCommandSpec
    /// 
    /// This is the core encoding algorithm that converts shell syntax into
    /// secure array format with proper pipe handling and redirection.
    pub fn parse_shell_to_array(&self, shell_cmd: &str) -> Result<ArrayCommandSpec> {
        // Use the shell-words crate to properly tokenize the shell command
        let tokens = shell_words::split(shell_cmd)
            .map_err(|e| crate::CupcakeError::Config(format!("Failed to parse shell command: {}", e)))?;

        if tokens.is_empty() {
            return Err(crate::CupcakeError::Config(
                "Empty command cannot be encoded".to_string()
            ));
        }

        // For this initial implementation, we'll handle simple commands and pipes
        // More complex shell constructs can be added in future versions
        
        if shell_cmd.contains('|') {
            self.parse_piped_command(&tokens, shell_cmd)
        } else if shell_cmd.contains('>') {
            self.parse_redirected_command(&tokens, shell_cmd)
        } else {
            self.parse_simple_command(&tokens)
        }
    }

    /// Parse a simple command without pipes or redirects
    fn parse_simple_command(&self, tokens: &[String]) -> Result<ArrayCommandSpec> {
        let command = vec![tokens[0].clone()];
        let args = if tokens.len() > 1 {
            Some(tokens[1..].to_vec())
        } else {
            None
        };

        Ok(ArrayCommandSpec {
            command,
            args,
            working_dir: None,
            env: None,
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        })
    }

    /// Parse a command with pipes
    fn parse_piped_command(&self, _tokens: &[String], shell_cmd: &str) -> Result<ArrayCommandSpec> {
        // Split on pipe characters and parse each segment
        let segments: Vec<&str> = shell_cmd.split('|').map(|s| s.trim()).collect();
        
        if segments.len() < 2 {
            return Err(crate::CupcakeError::Config(
                "Invalid pipe syntax".to_string()
            ));
        }

        // Parse the first command
        let first_tokens = shell_words::split(segments[0])
            .map_err(|e| crate::CupcakeError::Config(format!("Failed to parse first command: {}", e)))?;
        
        let command = vec![first_tokens[0].clone()];
        let args = if first_tokens.len() > 1 {
            Some(first_tokens[1..].to_vec())
        } else {
            None
        };

        // Parse pipe commands
        let mut pipe_commands = Vec::new();
        for segment in &segments[1..] {
            let pipe_tokens = shell_words::split(segment)
                .map_err(|e| crate::CupcakeError::Config(format!("Failed to parse pipe command: {}", e)))?;
            
            if !pipe_tokens.is_empty() {
                pipe_commands.push(crate::config::actions::PipeCommand {
                    cmd: pipe_tokens,
                });
            }
        }

        Ok(ArrayCommandSpec {
            command,
            args,
            working_dir: None,
            env: None,
            pipe: if pipe_commands.is_empty() { None } else { Some(pipe_commands) },
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        })
    }

    /// Parse a command with redirects
    fn parse_redirected_command(&self, _tokens: &[String], shell_cmd: &str) -> Result<ArrayCommandSpec> {
        // Simple redirect parsing - can be enhanced for complex cases
        let (cmd_part, redirect_part) = if let Some(_pos) = shell_cmd.find(" > ") {
            let parts: Vec<&str> = shell_cmd.splitn(2, " > ").collect();
            (parts[0], Some((false, parts[1].trim())))
        } else if let Some(_pos) = shell_cmd.find(" >> ") {
            let parts: Vec<&str> = shell_cmd.splitn(2, " >> ").collect();
            (parts[0], Some((true, parts[1].trim())))
        } else {
            (shell_cmd, None)
        };

        let cmd_tokens = shell_words::split(cmd_part)
            .map_err(|e| crate::CupcakeError::Config(format!("Failed to parse command: {}", e)))?;

        let command = vec![cmd_tokens[0].clone()];
        let args = if cmd_tokens.len() > 1 {
            Some(cmd_tokens[1..].to_vec())
        } else {
            None
        };

        let (redirect_stdout, append_stdout) = match redirect_part {
            Some((false, file)) => (Some(file.to_string()), None),
            Some((true, file)) => (None, Some(file.to_string())),
            None => (None, None),
        };

        Ok(ArrayCommandSpec {
            command,
            args,
            working_dir: None,
            env: None,
            pipe: None,
            redirect_stdout,
            append_stdout,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        })
    }

    /// Serialize ArrayCommandSpec to requested format
    pub fn serialize_array_spec(&self, spec: &ArrayCommandSpec) -> Result<String> {
        match self.format.as_str() {
            "yaml" => {
                serde_yaml_ng::to_string(spec)
                    .map_err(|e| crate::CupcakeError::Config(format!("YAML serialization failed: {}", e)))
            }
            "json" => {
                serde_json::to_string_pretty(spec)
                    .map_err(|e| crate::CupcakeError::Config(format!("JSON serialization failed: {}", e)))
            }
            _ => Err(crate::CupcakeError::Config(format!(
                "Unsupported format: {}. Use 'yaml' or 'json'.",
                self.format
            ))),
        }
    }

    /// Create full template with metadata and comments
    fn create_full_template(&self, spec: &ArrayCommandSpec) -> Result<String> {
        let serialized = self.serialize_array_spec(spec)?;
        
        let template = match self.format.as_str() {
            "yaml" => format!(
                "# Encoded shell command: {}\n# Generated by cupcake encode\n# This secure array format eliminates shell injection risks\n\n{}",
                self.command,
                serialized
            ),
            "json" => format!(
                "{{\"_metadata\": {{\"original_command\": \"{}\", \"generated_by\": \"cupcake encode\", \"note\": \"Secure array format eliminates shell injection risks\"}}, \"command_spec\": {}}}",
                self.command.replace('"', "\\\""),
                serialized
            ),
            _ => return Err(crate::CupcakeError::Config(format!(
                "Unsupported format: {}. Use 'yaml' or 'json'.",
                self.format
            ))),
        };

        Ok(template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_command_creation() {
        let cmd = EncodeCommand::new(
            "echo 'hello world'".to_string(),
            "yaml".to_string(),
            false,
        );

        assert_eq!(cmd.command, "echo 'hello world'");
        assert_eq!(cmd.format, "yaml");
        assert!(!cmd.template);
        assert_eq!(cmd.name(), "encode");
    }

    #[test]
    fn test_parse_simple_command() {
        let cmd = EncodeCommand::new(
            "echo hello".to_string(),
            "yaml".to_string(),
            false,
        );

        let tokens = vec!["echo".to_string(), "hello".to_string()];
        let spec = cmd.parse_simple_command(&tokens).unwrap();

        assert_eq!(spec.command, vec!["echo"]);
        assert_eq!(spec.args, Some(vec!["hello".to_string()]));
        assert!(spec.pipe.is_none());
        assert!(spec.redirect_stdout.is_none());
    }

    #[test]
    fn test_parse_piped_command() {
        let cmd = EncodeCommand::new(
            "ls -la | grep test".to_string(),
            "yaml".to_string(),
            false,
        );

        let tokens = vec!["ls".to_string(), "-la".to_string(), "|".to_string(), "grep".to_string(), "test".to_string()];
        let spec = cmd.parse_piped_command(&tokens, "ls -la | grep test").unwrap();

        assert_eq!(spec.command, vec!["ls"]);
        assert_eq!(spec.args, Some(vec!["-la".to_string()]));
        assert!(spec.pipe.is_some());
        
        let pipes = spec.pipe.unwrap();
        assert_eq!(pipes.len(), 1);
        assert_eq!(pipes[0].cmd, vec!["grep", "test"]);
    }

    #[test]
    fn test_parse_redirected_command() {
        let cmd = EncodeCommand::new(
            "echo hello > output.txt".to_string(),
            "yaml".to_string(),
            false,
        );

        let tokens = vec!["echo".to_string(), "hello".to_string(), ">".to_string(), "output.txt".to_string()];
        let spec = cmd.parse_redirected_command(&tokens, "echo hello > output.txt").unwrap();

        assert_eq!(spec.command, vec!["echo"]);
        assert_eq!(spec.args, Some(vec!["hello".to_string()]));
        assert_eq!(spec.redirect_stdout, Some("output.txt".to_string()));
        assert!(spec.append_stdout.is_none());
    }

    #[test]
    fn test_parse_append_redirected_command() {
        let cmd = EncodeCommand::new(
            "echo hello >> output.txt".to_string(),
            "yaml".to_string(),
            false,
        );

        let tokens = vec!["echo".to_string(), "hello".to_string(), ">>".to_string(), "output.txt".to_string()];
        let spec = cmd.parse_redirected_command(&tokens, "echo hello >> output.txt").unwrap();

        assert_eq!(spec.command, vec!["echo"]);
        assert_eq!(spec.args, Some(vec!["hello".to_string()]));
        assert_eq!(spec.append_stdout, Some("output.txt".to_string()));
        assert!(spec.redirect_stdout.is_none());
    }

    #[test]
    fn test_serialize_to_yaml() {
        let cmd = EncodeCommand::new(
            "echo hello".to_string(),
            "yaml".to_string(),
            false,
        );

        let spec = ArrayCommandSpec {
            command: vec!["echo".to_string()],
            args: Some(vec!["hello".to_string()]),
            working_dir: None,
            env: None,
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        };

        let yaml = cmd.serialize_array_spec(&spec).unwrap();
        assert!(yaml.contains("command:"));
        assert!(yaml.contains("- echo"));
        assert!(yaml.contains("args:"));
        assert!(yaml.contains("- hello"));
    }

    #[test]
    fn test_serialize_to_json() {
        let cmd = EncodeCommand::new(
            "echo hello".to_string(),
            "json".to_string(),
            false,
        );

        let spec = ArrayCommandSpec {
            command: vec!["echo".to_string()],
            args: Some(vec!["hello".to_string()]),
            working_dir: None,
            env: None,
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        };

        let json = cmd.serialize_array_spec(&spec).unwrap();
        assert!(json.contains("\"command\""));
        assert!(json.contains("[\n    \"echo\"\n  ]"));
        assert!(json.contains("\"args\""));
        assert!(json.contains("[\n    \"hello\"\n  ]"));
    }

    #[test]
    fn test_create_full_template_yaml() {
        let cmd = EncodeCommand::new(
            "echo hello".to_string(),
            "yaml".to_string(),
            true,
        );

        let spec = ArrayCommandSpec {
            command: vec!["echo".to_string()],
            args: Some(vec!["hello".to_string()]),
            working_dir: None,
            env: None,
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        };

        let template = cmd.create_full_template(&spec).unwrap();
        assert!(template.contains("# Encoded shell command: echo hello"));
        assert!(template.contains("# Generated by cupcake encode"));
        assert!(template.contains("# This secure array format eliminates shell injection risks"));
        assert!(template.contains("command:"));
        assert!(template.contains("- echo"));
    }

    #[test]
    fn test_invalid_format() {
        let cmd = EncodeCommand::new(
            "echo hello".to_string(),
            "xml".to_string(),
            false,
        );

        let spec = ArrayCommandSpec {
            command: vec!["echo".to_string()],
            args: Some(vec!["hello".to_string()]),
            working_dir: None,
            env: None,
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        };

        let result = cmd.serialize_array_spec(&spec);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported format: xml"));
    }

    #[test]
    fn test_empty_command() {
        let cmd = EncodeCommand::new(
            "".to_string(),
            "yaml".to_string(),
            false,
        );

        let result = cmd.parse_shell_to_array("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty command cannot be encoded"));
    }

    #[test]
    fn test_round_trip_encoding() {
        let test_commands = vec![
            "echo 'hello world'",
            "ls -la",
            "npm test | grep -v warning",
            "cat file.txt > output.txt",
            "echo hello >> log.txt",
        ];

        for shell_cmd in test_commands {
            let cmd = EncodeCommand::new(
                shell_cmd.to_string(),
                "yaml".to_string(),
                false,
            );

            // Should be able to parse without errors
            let result = cmd.parse_shell_to_array(shell_cmd);
            assert!(result.is_ok(), "Failed to parse: {}", shell_cmd);

            // Should be able to serialize without errors
            let spec = result.unwrap();
            let yaml_result = cmd.serialize_array_spec(&spec);
            assert!(yaml_result.is_ok(), "Failed to serialize: {}", shell_cmd);
        }
    }
}