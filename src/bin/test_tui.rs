#[cfg(feature = "tui")]
use cupcake::cli::tui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "tui")]
    {
        // Run the TUI init wizard
        tui::run_init_wizard().await?;
        Ok(())
    }

    #[cfg(not(feature = "tui"))]
    {
        eprintln!("TUI support is disabled. Rebuild with `--features tui` to test the TUI.");
        std::process::exit(1);
    }
}
