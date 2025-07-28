/// Terminal User Interface module for interactive commands
pub mod init;

use crate::Result;

/// Run the interactive init wizard
pub async fn run_init_wizard() -> Result<()> {
    init::run().await
}