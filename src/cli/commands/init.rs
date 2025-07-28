use super::CommandHandler;
use crate::Result;
use crate::cli::tui;

/// Handler for the `init` command
pub struct InitCommand {
    pub output: String,
    pub yes: bool,
    pub verbose: bool,
}

impl CommandHandler for InitCommand {
    fn execute(&self) -> Result<()> {
        // Launch the TUI wizard
        // Note: We need to use tokio runtime since our TUI is async
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| crate::CupcakeError::Config(format!("Failed to create async runtime: {}", e)))?;
        
        runtime.block_on(async {
            tui::run_init_wizard().await
        })
    }

    fn name(&self) -> &'static str {
        "init"
    }
}

impl InitCommand {
    /// Create new init command
    pub fn new(output: String, yes: bool, verbose: bool) -> Self {
        Self {
            output,
            yes,
            verbose,
        }
    }
}
