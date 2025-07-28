pub mod init;
pub mod inspect;
pub mod run;
pub mod sync;
pub mod validate;

use crate::Result;

/// Common trait for all command handlers
pub trait CommandHandler {
    /// Execute the command
    fn execute(&self) -> Result<()>;

    /// Get command name for logging
    fn name(&self) -> &'static str;

    /// Check if command requires root/admin privileges
    fn requires_privileges(&self) -> bool {
        false
    }
}

/// Command execution result
#[derive(Debug)]
pub enum CommandResult {
    /// Success with optional message
    Success(Option<String>),
    /// Error with message
    Error(String),
    /// Warning with message
    Warning(String),
}

impl CommandResult {
    /// Convert to exit code
    pub fn exit_code(&self) -> i32 {
        match self {
            CommandResult::Success(_) => 0,
            CommandResult::Error(_) => 1,
            CommandResult::Warning(_) => 0,
        }
    }

    /// Get message if any
    pub fn message(&self) -> Option<&str> {
        match self {
            CommandResult::Success(msg) => msg.as_deref(),
            CommandResult::Error(msg) => Some(msg),
            CommandResult::Warning(msg) => Some(msg),
        }
    }
}
