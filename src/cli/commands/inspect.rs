use super::CommandHandler;
use crate::config::loader::PolicyLoader;
use crate::config::types::ComposedPolicy;
use crate::config::conditions::Condition;
use crate::Result;
use std::path::Path;

/// Handler for the `inspect` command
pub struct InspectCommand {
    pub config: String,
}

impl CommandHandler for InspectCommand {
    fn execute(&self) -> Result<()> {
        let mut loader = PolicyLoader::new();

        // Load policies using the same logic as RunCommand
        let policies = if !self.config.is_empty() {
            // User specified a config file - load from that file
            let config_path = Path::new(&self.config);
            loader.load_from_config_file(config_path)?
        } else {
            // No config specified - use auto-discovery
            let current_dir = std::env::current_dir().map_err(|e| {
                crate::CupcakeError::Config(format!("Failed to get current directory: {}", e))
            })?;
            loader.load_and_compose_policies(&current_dir)?
        };

        if policies.is_empty() {
            println!("No policies found.");
            return Ok(());
        }

        // Print compact table format
        self.print_policies_table(&policies);

        Ok(())
    }

    fn name(&self) -> &'static str {
        "inspect"
    }
}

impl InspectCommand {
    /// Create new inspect command
    pub fn new(config: String) -> Self {
        Self { config }
    }

    /// Print policies in compact table format
    fn print_policies_table(&self, policies: &[ComposedPolicy]) {
        // Calculate column widths
        let name_width = policies.iter()
            .map(|p| p.name.len())
            .max()
            .unwrap_or(4)
            .max(4); // "NAME"

        let event_width = policies.iter()
            .map(|p| p.hook_event.to_string().len())
            .max()
            .unwrap_or(5)
            .max(5); // "EVENT"

        let tool_width = policies.iter()
            .map(|p| p.matcher.len())
            .max()
            .unwrap_or(4)
            .max(4); // "TOOL"

        let action_width = policies.iter()
            .map(|p| self.format_action_type(p).len())
            .max()
            .unwrap_or(6)
            .max(6); // "ACTION"

        // Print header
        println!(
            "{:<name_width$} {:<event_width$} {:<tool_width$} {:<action_width$} CONDITIONS",
            "NAME", "EVENT", "TOOL", "ACTION",
            name_width = name_width,
            event_width = event_width,
            tool_width = tool_width,
            action_width = action_width
        );

        // Print separator
        println!(
            "{} {} {} {} {}",
            "-".repeat(name_width),
            "-".repeat(event_width),
            "-".repeat(tool_width),
            "-".repeat(action_width),
            "-".repeat(10) // CONDITIONS
        );

        // Print policies
        for policy in policies {
            println!(
                "{:<name_width$} {:<event_width$} {:<tool_width$} {:<action_width$} {}",
                policy.name,
                policy.hook_event.to_string(),
                policy.matcher,
                self.format_action_type(policy),
                self.format_conditions(&policy.conditions),
                name_width = name_width,
                event_width = event_width,
                tool_width = tool_width,
                action_width = action_width
            );
        }

        println!("\nTotal: {} policies", policies.len());
    }

    /// Format action type as a short string
    fn format_action_type(&self, policy: &ComposedPolicy) -> String {
        match &policy.action {
            crate::config::actions::Action::ProvideFeedback { .. } => "provide_feedback".to_string(),
            crate::config::actions::Action::BlockWithFeedback { .. } => "block_with_feedback".to_string(),
            crate::config::actions::Action::Approve { .. } => "approve".to_string(),
            crate::config::actions::Action::RunCommand { .. } => "run_command".to_string(),
            crate::config::actions::Action::UpdateState { .. } => "update_state".to_string(),
            crate::config::actions::Action::Conditional { .. } => "conditional".to_string(),
        }
    }

    /// Format conditions as a compact string
    fn format_conditions(&self, conditions: &[Condition]) -> String {
        if conditions.is_empty() {
            return "always".to_string();
        }

        if conditions.len() == 1 {
            self.format_single_condition(&conditions[0])
        } else {
            // Multiple conditions - show count and first one
            format!("{} conditions: {}", conditions.len(), self.format_single_condition(&conditions[0]))
        }
    }

    /// Format a single condition as a compact string
    #[allow(clippy::only_used_in_recursion)]
    fn format_single_condition(&self, condition: &Condition) -> String {
        match condition {
            Condition::Pattern { field, regex } => {
                format!("{} ~ \"{}\"", field, regex)
            }
            Condition::Match { field, value } => {
                format!("{} = \"{}\"", field, value)
            }
            Condition::Check { spec, expect_success } => {
                // TODO: Improve display of CommandSpec in Phase 2
                let command_display = match spec.as_ref() {
                    crate::config::actions::CommandSpec::Array(array_spec) => {
                        let mut parts = array_spec.command.clone();
                        if let Some(args) = &array_spec.args {
                            parts.extend(args.clone());
                        }
                        parts.join(" ")
                    }
                    crate::config::actions::CommandSpec::String(string_spec) => {
                        string_spec.command.clone()
                    }
                    crate::config::actions::CommandSpec::Shell(shell_spec) => {
                        format!("shell: {}", shell_spec.script.chars().take(50).collect::<String>())
                    }
                };
                if *expect_success {
                    format!("check \"{}\"", command_display)
                } else {
                    format!("check !\"{}\"", command_display)
                }
            }
            Condition::And { conditions } => {
                if conditions.len() <= 2 {
                    format!(
                        "({} AND {})",
                        self.format_single_condition(&conditions[0]),
                        self.format_single_condition(&conditions[1])
                    )
                } else {
                    format!("({} AND {} more)", 
                        self.format_single_condition(&conditions[0]),
                        conditions.len() - 1
                    )
                }
            }
            Condition::Or { conditions } => {
                if conditions.len() <= 2 {
                    format!(
                        "({} OR {})",
                        self.format_single_condition(&conditions[0]),
                        self.format_single_condition(&conditions[1])
                    )
                } else {
                    format!("({} OR {} more)", 
                        self.format_single_condition(&conditions[0]),
                        conditions.len() - 1
                    )
                }
            }
            Condition::Not { condition } => {
                format!("NOT {}", self.format_single_condition(condition))
            }
        }
    }
}