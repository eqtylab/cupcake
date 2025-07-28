use cupcake::cli::tui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Run the TUI init wizard
    tui::run_init_wizard().await?;
    Ok(())
}