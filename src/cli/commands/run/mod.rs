mod context;
mod engine;
mod parser;

pub use self::context::ExecutionContextBuilder;
pub use self::engine::EngineRunner;
pub use self::parser::HookEventParser;

use super::CommandHandler;
use crate::config::loader::PolicyLoader;
use crate::engine::response::{EngineDecision, ResponseHandler};
use crate::Result;

/// Handler for the `run` command
pub struct RunCommand {
    pub event: String,
    pub config: String,
    pub debug: bool,
}

impl RunCommand {
    pub fn new(event: String, config: String, debug: bool) -> Self {
        Self {
            event,
            config,
            debug,
        }
    }

    fn log_debug(&self, message: &str) {
        if self.debug {
            eprintln!("Debug: {}", message);
        }
    }

    fn append_debug_log(&self, message: &str) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/cupcake-debug.log")
        {
            use std::io::Write;
            let timestamp = chrono::Local::now()
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string();
            let _ = writeln!(file, "[{}] {}", timestamp, message);
        }
    }

    fn load_configuration(&self) -> Result<crate::config::loader::LoadedConfiguration> {
        let mut loader = PolicyLoader::new();

        if !self.config.is_empty() {
            let config_path = std::path::Path::new(&self.config);
            loader.load_configuration(config_path)
        } else {
            let current_dir = std::env::current_dir()?;
            loader.load_configuration_from_directory(&current_dir)
        }
    }
}

impl CommandHandler for RunCommand {
    fn execute(&self) -> Result<()> {
        // Log invocation
        self.append_debug_log(&format!(
            "Cupcake invoked - Event: {}, Config: {}, Debug: {}",
            self.event, self.config, self.debug
        ));

        self.log_debug(&format!("Event: {}", self.event));
        self.log_debug(&format!("Config file: {}", self.config));

        // Parse hook event
        let parser = HookEventParser::new(self.debug);
        let hook_event = match parser.parse_from_stdin() {
            Ok(event) => event,
            Err(e) => {
                eprintln!("Error reading hook event: {}", e);
                self.log_debug("Graceful degradation - allowing operation due to input error");
                self.append_debug_log(&format!("ERROR reading hook event: {}", e));
                std::process::exit(0);
            }
        };

        self.log_debug(&format!("Parsed hook event: {:?}", hook_event));

        // Load configuration
        let configuration = match self.load_configuration() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Error loading configuration: {}", e);
                self.log_debug(
                    "Graceful degradation - allowing operation due to configuration loading error",
                );
                std::process::exit(0);
            }
        };

        self.log_debug(&format!(
            "Loaded {} composed policies",
            configuration.policies.len()
        ));

        // Build contexts
        let context_builder = ExecutionContextBuilder::new();
        let evaluation_context = context_builder.build_evaluation_context(&hook_event);
        let action_context = context_builder.build_action_context(&hook_event);

        // Run engine
        let mut engine = EngineRunner::new(configuration.settings, self.debug);
        let result = match engine.run(
            &configuration.policies,
            &hook_event,
            &evaluation_context,
            &action_context,
        ) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Error during policy evaluation: {}", e);
                self.log_debug("Graceful degradation - allowing operation due to evaluation error");
                ResponseHandler::new(self.debug).send_response_for_hook(
                    EngineDecision::Allow { reason: None },
                    hook_event.event_name(),
                );
            }
        };

        // Handle special cases for UserPromptSubmit
        if hook_event.event_name() == "UserPromptSubmit" {
            ResponseHandler::new(self.debug)
                .send_user_prompt_response(result.final_decision, result.context_to_inject);
        }

        // Handle soft feedback for non-UserPromptSubmit
        if hook_event.event_name() != "UserPromptSubmit"
            && matches!(result.final_decision, EngineDecision::Allow { .. })
            && !result.feedback_messages.is_empty()
        {
            let feedback_output = result.feedback_messages.join("\n");
            println!("{}", feedback_output);
        }

        // Send final response
        let response_decision = if hook_event.event_name() == "PostToolUse"
            && matches!(result.final_decision, EngineDecision::Allow { .. })
            && !result.feedback_messages.is_empty()
        {
            EngineDecision::Block {
                feedback: result.feedback_messages.join("\n"),
            }
        } else {
            result.final_decision
        };

        ResponseHandler::new(self.debug)
            .send_response_for_hook(response_decision, hook_event.event_name());
    }

    fn name(&self) -> &'static str {
        "run"
    }
}
