use std::path::PathBuf;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use crate::cli::tui::init::state::{RuleFile, ExtractedRule};

/// All possible events in the application
#[derive(Debug)]
pub enum AppEvent {
    // Input events
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),

    // Async task events - File discovery
    FileFound(RuleFile),
    ScanProgress(f64),
    ScanComplete,

    // Async task events - Extraction
    ExtractionStarted { file: PathBuf },
    ExtractionProgress { file: PathBuf, progress: f64 },
    ExtractionComplete { file: PathBuf, rules: Vec<ExtractedRule> },
    ExtractionFailed { file: PathBuf, error: String },

    // Async task events - Compilation
    CompilationProgress { phase: usize, progress: f64 },
    CompilationPhaseComplete { phase: usize },
    CompilationPhaseFailed { phase: usize, error: String },
    CompilationLog(String),

    // UI events
    Tick, // for smooth animations and progress bars
}