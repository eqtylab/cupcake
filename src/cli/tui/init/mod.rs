/// Interactive init wizard implementation
pub mod app;
pub mod claude_settings;
pub mod components;
pub mod discovery;
pub mod events;
pub mod extraction;
pub mod modal;
pub mod preview;
pub mod screens;
pub mod state;
pub mod yaml_writer;

use crate::Result;

/// Entry point for the init wizard
pub async fn run() -> Result<()> {
    let app = app::App::new();
    app.run().await
}
