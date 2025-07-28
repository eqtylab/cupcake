/// Interactive init wizard implementation
pub mod app;
pub mod state;
pub mod events;
pub mod screens;
pub mod components;
pub mod discovery;
pub mod preview;
pub mod modal;
pub mod yaml_writer;
pub mod claude_settings;
pub mod extraction;

use crate::Result;

/// Entry point for the init wizard
pub async fn run() -> Result<()> {
    let app = app::App::new();
    app.run().await
}