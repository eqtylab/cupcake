mod context;
mod engine;
mod parser;

pub use self::context::ExecutionContextBuilder;
pub use self::engine::EngineRunner;
pub use self::parser::HookEventParser;

use super::CommandHandler;
use crate::config::loader::PolicyLoader;
use crate::engine::events::AgentEvent;
use crate::engine::response::{
    claude_code::ClaudeCodeResponseBuilder, ResponseHandler,
};
use crate::{Result, tracing::{debug, info}};

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

    fn append_debug_log(&self, message: &str) {
        // Log to both tracing and file for backward compatibility
        info!(message);
        
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/cupcake-debug.log")
        {
            use std::io::Write;
            let timestamp = chrono::Local::now()
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string();
            let _ = writeln!(file, "[{timestamp}] {message}");
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

        debug!(event = %self.event, "Processing hook event");
        debug!(config = %self.config, "Using config file");

        // Parse hook event
        let parser = HookEventParser::new(self.debug);
        let hook_event = match parser.parse_from_stdin() {
            Ok(event) => event,
            Err(e) => {
                // Use the event type from command line for proper error response format
                crate::cli::error_handler::handle_run_command_error_with_type(e, &self.event);
            }
        };

        debug!(?hook_event, "Parsed hook event");

        // Load configuration
        let configuration = match self.load_configuration() {
            Ok(config) => config,
            Err(e) => {
                // Use the event type from command line for proper error response format
                crate::cli::error_handler::handle_run_command_error_with_type(e, &self.event);
            }
        };

        debug!(policy_count = configuration.policies.len(), "Loaded composed policies");

        // Run engine - now creates its own contexts internally
        let mut engine = EngineRunner::new(configuration.settings, self.debug);

        let result = match engine.run(&configuration.policies, &hook_event) {
            Ok(result) => result,
            Err(e) => {
                // Use the event type from command line for proper error response format
                crate::cli::error_handler::handle_run_command_error_with_type(e, &self.event);
            }
        };

        // Handle context injection based on injection mode
        // Special case: PreCompact always uses stdout regardless of use_stdout setting
        if hook_event.event_name() == "PreCompact" && !result.context_to_inject.is_empty() {
            let combined_instructions = result.context_to_inject.join("\n\n");
            println!("{combined_instructions}");
            std::process::exit(0);
        }
        
        // For other events, check injection mode
        // IMPORTANT: Block decisions always use JSON regardless of injection mode
        if let Some(engine::InjectionMode::Stdout) = result.injection_mode {
            if !matches!(result.final_decision, crate::engine::response::EngineDecision::Block { .. }) {
                // In stdout mode for non-blocking decisions, output context and exit
                let combined_context = result.context_to_inject.join("\n");
                println!("{combined_context}");
                std::process::exit(0);
            }
        }
        
        // For JSON mode, Block decisions, or when no injection mode is set, continue to JSON response

        // Use the new modular response builder for all other cases
        // Extract the HookEvent from AgentEvent
        let AgentEvent::ClaudeCode(claude_event) = &hook_event;

        let response = ClaudeCodeResponseBuilder::build_response(
            &result.final_decision,
            claude_event,
            if result.context_to_inject.is_empty() {
                None
            } else {
                Some(result.context_to_inject)
            },
            result.suppress_output,
        );

        ResponseHandler::new(self.debug).send_json_response(response);
    }

    fn name(&self) -> &'static str {
        "run"
    }
}
