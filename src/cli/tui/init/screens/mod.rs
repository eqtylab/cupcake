/// Screen modules for the init wizard
pub mod discovery;
pub mod extraction;
pub mod review;
pub mod compilation;
pub mod success;

use ratatui::Frame;

/// Trait for screens in the wizard
pub trait Screen {
    /// Render the screen
    fn render(&mut self, frame: &mut Frame);
    
    /// Get help text for this screen
    fn help_text(&self) -> &str;
}