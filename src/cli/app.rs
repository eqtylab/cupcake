use clap::{Parser, Subcommand};

/// Cupcake: Deterministic policy enforcement for Claude Code
#[derive(Parser)]
#[command(name = "cupcake")]
#[command(version = "0.1.0")]
#[command(about = "Deterministic policy enforcement for Claude Code")]
#[command(
    long_about = "Cupcake transforms natural language rules in CLAUDE.md files into enforceable policies via Claude Code's hook system."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Interactive policy generation from CLAUDE.md files
    Init {
        /// Output directory for generated policies
        #[arg(short, long, default_value = "guardrails")]
        output: String,

        /// Skip interactive confirmation
        #[arg(short, long)]
        yes: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Runtime enforcement (called by hooks)
    Run {
        /// Hook event type
        #[arg(long)]
        event: String,

        /// Configuration file path (automatically discovered from guardrails/cupcake.yaml)
        #[arg(long, default_value = "")]
        config: String,

        /// Enable debug output
        #[arg(long)]
        debug: bool,
    },

    /// Updates Claude Code hooks in settings.json
    Sync {
        /// Claude Code settings file path
        #[arg(short, long)]
        settings_path: Option<String>,

        /// Dry run - show what would be changed without making changes
        #[arg(short, long)]
        dry_run: bool,

        /// Force overwrite existing hooks
        #[arg(short, long)]
        force: bool,
    },

    /// Validates policy file syntax
    Validate {
        /// Path to policy directory (automatically discovered from guardrails/cupcake.yaml)
        #[arg(default_value = "")]
        policy_file: String,

        /// Strict validation mode
        #[arg(short, long)]
        strict: bool,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Views audit logs
    Audit {
        /// Number of recent entries to show
        #[arg(short, long)]
        tail: Option<usize>,

        /// Follow mode - watch for new entries
        #[arg(short, long)]
        follow: bool,

        /// Filter by session ID
        #[arg(long)]
        session: Option<String>,

        /// Filter by hook event type
        #[arg(long)]
        event: Option<String>,

        /// Output format (text, json)
        #[arg(long, default_value = "text")]
        format: String,

        /// Clear audit log
        #[arg(long)]
        clear: bool,
    },

    /// Inspect loaded policies in compact table format
    Inspect {
        /// Configuration file path (automatically discovered from guardrails/cupcake.yaml)
        #[arg(long, default_value = "")]
        config: String,
    },

}

impl Commands {
    /// Get the command name as a string
    pub fn name(&self) -> &'static str {
        match self {
            Commands::Init { .. } => "init",
            Commands::Run { .. } => "run",
            Commands::Sync { .. } => "sync",
            Commands::Validate { .. } => "validate",
            Commands::Audit { .. } => "audit",
            Commands::Inspect { .. } => "inspect",
        }
    }

    /// Check if this command requires a policy file
    pub fn requires_policy_file(&self) -> bool {
        matches!(self, Commands::Run { .. } | Commands::Validate { .. } | Commands::Inspect { .. })
    }

    /// Check if this command modifies files
    pub fn modifies_files(&self) -> bool {
        matches!(self, Commands::Init { .. } | Commands::Sync { .. })
    }

    /// Check if this command requires elevated permissions
    pub fn requires_permissions(&self) -> bool {
        matches!(self, Commands::Sync { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::parse_from(&["cupcake", "init", "--output", "test-guardrails", "--yes"]);

        match cli.command {
            Commands::Init {
                output,
                yes,
                verbose,
            } => {
                assert_eq!(output, "test-guardrails");
                assert!(yes);
                assert!(!verbose);
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_run_command_defaults() {
        let cli = Cli::parse_from(&["cupcake", "run", "--event", "PreToolUse"]);

        match cli.command {
            Commands::Run {
                event,
                config,
                debug,
            } => {
                assert_eq!(event, "PreToolUse");
                assert_eq!(config, ""); // Auto-discovery mode
                assert!(!debug);
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_validate_command() {
        let cli = Cli::parse_from(&["cupcake", "validate", "my-guardrails", "--strict"]);

        match cli.command {
            Commands::Validate {
                policy_file,
                strict,
                format,
            } => {
                assert_eq!(policy_file, "my-guardrails");
                assert!(strict);
                assert_eq!(format, "text");
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_audit_command() {
        let cli = Cli::parse_from(&[
            "cupcake",
            "audit",
            "--tail",
            "100",
            "--session",
            "test-session",
        ]);

        match cli.command {
            Commands::Audit {
                tail,
                follow,
                session,
                event,
                format,
                clear,
            } => {
                assert_eq!(tail, Some(100));
                assert!(!follow);
                assert_eq!(session, Some("test-session".to_string()));
                assert_eq!(event, None);
                assert_eq!(format, "text");
                assert!(!clear);
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_sync_command() {
        let cli = Cli::parse_from(&["cupcake", "sync", "--dry-run", "--force"]);

        match cli.command {
            Commands::Sync {
                settings_path,
                dry_run,
                force,
            } => {
                assert_eq!(settings_path, None);
                assert!(dry_run);
                assert!(force);
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_inspect_command() {
        let cli = Cli::parse_from(&["cupcake", "inspect", "--config", "my-config.yaml"]);

        match cli.command {
            Commands::Inspect { config } => {
                assert_eq!(config, "my-config.yaml");
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_inspect_command_defaults() {
        let cli = Cli::parse_from(&["cupcake", "inspect"]);

        match cli.command {
            Commands::Inspect { config } => {
                assert_eq!(config, ""); // Auto-discovery mode
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_command_properties() {
        let init_cmd = Commands::Init {
            output: "test-guardrails".to_string(),
            yes: false,
            verbose: false,
        };

        assert_eq!(init_cmd.name(), "init");
        assert!(!init_cmd.requires_policy_file());
        assert!(init_cmd.modifies_files());
        assert!(!init_cmd.requires_permissions());

        let run_cmd = Commands::Run {
            event: "PreToolUse".to_string(),
            config: "".to_string(), // Auto-discovery
            debug: false,
        };

        assert_eq!(run_cmd.name(), "run");
        assert!(run_cmd.requires_policy_file());
        assert!(!run_cmd.modifies_files());
        assert!(!run_cmd.requires_permissions());

        let sync_cmd = Commands::Sync {
            settings_path: None,
            dry_run: false,
            force: false,
        };

        assert_eq!(sync_cmd.name(), "sync");
        assert!(!sync_cmd.requires_policy_file());
        assert!(sync_cmd.modifies_files());
        assert!(sync_cmd.requires_permissions());

    }

}
