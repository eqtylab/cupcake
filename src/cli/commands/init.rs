use super::CommandHandler;
use crate::Result;

#[cfg(feature = "tui")]
use crate::cli::tui;

/// Handler for the `init` command
pub struct InitCommand {
    pub output: String,
    pub yes: bool,
    pub verbose: bool,
}

impl CommandHandler for InitCommand {
    fn execute(&self) -> Result<()> {
        #[cfg(feature = "tui")]
        {
            // Launch the TUI wizard
            // Note: We need to use tokio runtime since our TUI is async
            let runtime = tokio::runtime::Runtime::new().map_err(|e| {
                crate::CupcakeError::Config(format!("Failed to create async runtime: {}", e))
            })?;

            runtime.block_on(async { tui::run_init_wizard().await })
        }

        #[cfg(not(feature = "tui"))]
        {
            Err(crate::CupcakeError::Config(
                "TUI support is disabled. Rebuild with `--features tui` to use the interactive init wizard.".into(),
            ))
        }
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
