use std::collections::HashSet;
use std::path::PathBuf;
use tui_input::Input;

/// Main state machine for the wizard
#[derive(Debug)]
pub enum WizardState {
    Landing(LandingState),
    Discovery(DiscoveryState),
    Extraction(ExtractionState),
    Review(ReviewState),
    Compilation(CompilationState),
    Success(SuccessState),
}

/// State for landing screen
#[derive(Debug)]
pub struct LandingState {
    pub auto_discovery: bool,
}

impl Default for LandingState {
    fn default() -> Self {
        Self {
            auto_discovery: true,
        }
    }
}

/// State for file discovery screen
#[derive(Debug, Default)]
pub struct DiscoveryState {
    // File discovery
    pub files: Vec<RuleFile>,
    pub selected: HashSet<PathBuf>,
    pub scan_complete: bool,
    pub scan_progress: f64,

    // UI state
    pub selected_index: usize,
    pub focused_pane: Pane,
    pub preview_content: Option<String>,
    pub preview_scroll_offset: u16,

    // Custom prompt modal
    pub show_custom_prompt: bool,
    pub custom_prompt_input: Input,
}

/// State for extraction progress screen
#[derive(Debug, Default)]
pub struct ExtractionState {
    pub tasks: Vec<ExtractionTask>,
    pub overall_progress: f64,
    pub custom_instructions: Option<String>,
    /// All extracted rules from completed tasks
    pub extracted_rules: Vec<ExtractedRule>,
    /// Track task start times for elapsed calculation
    pub task_start_times: std::collections::HashMap<PathBuf, std::time::Instant>,
    /// When compilation started (unix timestamp in ms)
    pub compilation_started_at: u64,
    /// When compilation completed (unix timestamp in ms)
    pub compilation_completed_at: u64,
    /// Whether compilation is complete
    pub compilation_complete: bool,
    /// Number of rules after compilation/deduplication
    pub compiled_rule_count: usize,
}

/// State for rule review screen
#[derive(Debug, Default)]
pub struct ReviewState {
    pub rules: Vec<ExtractedRule>,
    pub selected: HashSet<usize>,
    pub filtered_indices: Vec<usize>,

    // UI state
    pub selected_index: usize,
    pub selected_line: usize,  // Line index in the rendered list
    pub search_input: Input,
    pub search_active: bool,
    pub expanded_sections: HashSet<String>,

    // Edit modal
    pub editing_rule: Option<usize>,
    pub edit_form: RuleEditForm,
}

/// State for compilation screen
#[derive(Debug, Default)]
pub struct CompilationState {
    pub phases: Vec<CompilationPhase>,
    pub current_phase: usize,
    pub overall_progress: f64,
    pub show_logs: bool,
    pub logs: Vec<String>,
    /// Rule counts for success screen
    pub critical_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

/// State for success screen
#[derive(Debug)]
pub struct SuccessState {
    pub total_rules: usize,
    pub critical_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub config_location: PathBuf,
}

/// File discovered during scanning
#[derive(Debug, Clone)]
pub struct RuleFile {
    pub path: PathBuf,
    pub agent: Agent,
    pub is_directory: bool,
    pub children: Vec<PathBuf>,
}

/// Known agent types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agent {
    Claude,
    Cursor,
    Windsurf,
    Kiro,
    Copilot,
    Aider,
    Gemini,
}

impl Agent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Agent::Claude => "Claude",
            Agent::Cursor => "Cursor",
            Agent::Windsurf => "Windsurf",
            Agent::Kiro => "Kiro",
            Agent::Copilot => "Copilot",
            Agent::Aider => "Aider",
            Agent::Gemini => "Gemini",
        }
    }
}

/// Which pane is focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    FileList,
    Preview,
}

impl Default for Pane {
    fn default() -> Self {
        Pane::FileList
    }
}

/// Task for extracting rules from a file
#[derive(Debug)]
pub struct ExtractionTask {
    pub file_path: PathBuf,
    pub file_name: String,
    pub status: TaskStatus,
    pub progress: f64,
    pub elapsed_ms: u64,
    pub rules_found: usize,
}

/// Status of an extraction task
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Queued,
    InProgress,
    Complete,
    Failed(String),
}

/// An extracted rule
#[derive(Debug, Clone)]
pub struct ExtractedRule {
    pub id: usize,
    pub source_file: PathBuf,
    pub description: String,
    pub hook_description: String,  // What the hook will do when triggered
    pub severity: Severity,
    pub category: String,
    pub when: String,
    pub block_on_violation: bool,
    pub policy_decision: PolicyDecision,
}

/// Rule severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    High,
    Medium,
    Low,
}

/// Decision about whether to convert rule to policy
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    pub to_policy: bool,
    pub rationale: String,
}

/// Form for editing a rule
#[derive(Debug, Default)]
pub struct RuleEditForm {
    pub description: Input,
    pub severity: Severity,
    pub category: String,
    pub when: String,
    pub block_on_violation: bool,
    pub current_field: FormField,
}

/// Which field is focused in the edit form
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Description,
    Severity,
    Category,
    When,
    BlockOnViolation,
}

impl Default for FormField {
    fn default() -> Self {
        FormField::Description
    }
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Medium
    }
}

/// Compilation phase information
#[derive(Debug)]
pub struct CompilationPhase {
    pub name: String,
    pub status: PhaseStatus,
    pub details: Vec<String>,
    pub elapsed_ms: u64,
}

/// Status of a compilation phase
#[derive(Debug, Clone, PartialEq)]
pub enum PhaseStatus {
    Pending,
    InProgress,
    Complete,
    Failed(String),
}

/// State transitions for the wizard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateTransition {
    Continue,      // Go to next screen
    Back,          // Go to previous screen
    Skip,          // Skip optional screen
    Quit,          // Exit wizard
}

impl WizardState {
    /// Get the name of the current state
    pub fn name(&self) -> &'static str {
        match self {
            WizardState::Landing(_) => "Welcome",
            WizardState::Discovery(_) => "Discovery",
            WizardState::Extraction(_) => "Extraction",
            WizardState::Review(_) => "Review",
            WizardState::Compilation(_) => "Compilation",
            WizardState::Success(_) => "Success",
        }
    }

    /// Check if we can go back from this state
    pub fn can_go_back(&self) -> bool {
        !matches!(self, WizardState::Success(_))
    }

    /// Check if this state allows quitting
    pub fn can_quit(&self) -> bool {
        !matches!(self, WizardState::Compilation(_))
    }
}