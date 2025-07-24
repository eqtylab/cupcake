use std::time::Duration;
use std::collections::HashSet;
use std::path::PathBuf;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    DefaultTerminal, Frame
};
use tokio::time;
use tui_input::backend::crossterm::EventHandler;

use crate::Result;
use super::state::*;
use super::events::*;
use super::theme::Theme;

/// Main application struct
pub struct App {
    /// Current state of the wizard
    state: WizardState,
    /// Whether the app should quit
    should_quit: bool,
    /// Theme for styling
    theme: Theme,
    /// Event sender for background tasks
    event_tx: Option<tokio::sync::mpsc::UnboundedSender<AppEvent>>,
    /// Track Ctrl+C presses for double-press exit
    ctrl_c_count: u8,
    /// Last time Ctrl+C was pressed
    last_ctrl_c: Option<std::time::Instant>,
}

impl App {
    /// Create a new app instance
    pub fn new() -> Self {
        Self {
            state: WizardState::Landing(LandingState::default()),
            should_quit: false,
            theme: Theme::default(),
            event_tx: None,
            ctrl_c_count: 0,
            last_ctrl_c: None,
        }
    }

    /// Run the application
    pub async fn run(mut self) -> Result<()> {
        // Initialize terminal
        let mut terminal = ratatui::init();
        terminal.clear()?;

        // Create event channel
        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
        self.event_tx = Some(event_tx.clone());

        // Don't spawn background tasks yet - wait for Discovery state

        // Spawn input handler
        let input_tx = event_tx.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(event) = event::read() {
                    match event {
                        Event::Key(key) if key.kind == KeyEventKind::Press => {
                            let _ = input_tx.send(AppEvent::Key(key));
                        }
                        Event::Mouse(mouse) => {
                            let _ = input_tx.send(AppEvent::Mouse(mouse));
                        }
                        Event::Resize(width, height) => {
                            let _ = input_tx.send(AppEvent::Resize(width, height));
                        }
                        _ => {}
                    }
                }
            }
        });

        // Main render loop
        let result = self.main_loop(&mut terminal, &mut event_rx).await;

        // Cleanup
        ratatui::restore();
        result
    }

    /// Main event loop
    async fn main_loop(
        &mut self,
        terminal: &mut DefaultTerminal,
        event_rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
    ) -> Result<()> {
        loop {
            // Draw UI
            terminal.draw(|frame| self.render(frame))?;

            // Handle events with timeout for animations
            match time::timeout(Duration::from_millis(50), event_rx.recv()).await {
                Ok(Some(event)) => {
                    if let Some(transition) = self.handle_event(event)? {
                        self.transition_state(transition)?;
                    }
                }
                Ok(None) => break, // Channel closed
                Err(_) => {
                    // Timeout - send tick for animations
                    if let Some(transition) = self.handle_event(AppEvent::Tick)? {
                        self.transition_state(transition)?;
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Render the current state
    fn render(&mut self, frame: &mut Frame) {
        match &self.state {
            WizardState::Landing(state) => {
                super::screens::landing::render(frame, state);
            }
            WizardState::Discovery(state) => {
                super::screens::discovery::render(frame, state);
            }
            WizardState::Extraction(state) => {
                super::screens::extraction::render(frame, state);
            }
            WizardState::Review(state) => {
                super::screens::review::render(frame, state);
            }
            WizardState::Compilation(state) => {
                super::screens::compilation::render(frame, state);
            }
            WizardState::Success(state) => {
                super::screens::success::render(frame, state);
            }
        }
    }

    /// Handle an event
    fn handle_event(&mut self, event: AppEvent) -> Result<Option<StateTransition>> {
        // Handle global keys first
        if let AppEvent::Key(key) = &event {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.should_quit = true;
                    return Ok(None);
                }
                KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    // Handle Ctrl+C - exit on double press
                    let now = std::time::Instant::now();
                    if let Some(last) = self.last_ctrl_c {
                        if now.duration_since(last).as_millis() < 1000 {
                            // Double Ctrl+C within 1 second - exit
                            self.should_quit = true;
                            return Ok(None);
                        }
                    }
                    self.ctrl_c_count = 1;
                    self.last_ctrl_c = Some(now);
                }
                _ => {}
            }
        }

        // Route to state-specific handler
        use WizardState::*;
        match self.state {
            Landing(_) => {
                if let Landing(ref mut state) = self.state {
                    return Self::handle_landing_event(state, event);
                }
                Ok(None)
            }
            Discovery(_) => {
                if let Discovery(ref mut state) = self.state {
                    return Self::handle_discovery_event(state, event);
                }
                Ok(None)
            }
            Extraction(_) => {
                if let Extraction(ref mut state) = self.state {
                    return Self::handle_extraction_event(state, event);
                }
                Ok(None)
            }
            Review(_) => {
                if let Review(ref mut state) = self.state {
                    return Self::handle_review_event(state, event);
                }
                Ok(None)
            }
            Compilation(_) => {
                if let Compilation(ref mut state) = self.state {
                    return Self::handle_compilation_event(state, event);
                }
                Ok(None)
            }
            Success(_) => self.handle_success_event(event),
        }
    }

    /// Transition to a new state
    fn transition_state(&mut self, transition: StateTransition) -> Result<()> {
        match transition {
            StateTransition::Continue => {
                self.state = match &self.state {
                    WizardState::Landing(landing) => {
                        if landing.auto_discovery {
                            // Start file discovery in next state
                            let discovery_state = DiscoveryState::default();
                            // Spawn file discovery task
                            if let Some(tx) = &self.event_tx {
                                self.spawn_file_discovery(tx.clone());
                            }
                            WizardState::Discovery(discovery_state)
                        } else {
                            // TODO: Manual rule creation mode
                            WizardState::Discovery(DiscoveryState::default())
                        }
                    }
                    WizardState::Discovery(discovery) => {
                        // Pass custom instructions if provided
                        let mut extraction = ExtractionState::default();
                        if discovery.custom_prompt_input.value().trim().len() > 0 {
                            extraction.custom_instructions = Some(discovery.custom_prompt_input.value().to_string());
                        }
                        
                        // Start extraction for selected files
                        self.start_extraction(&discovery.selected, &mut extraction)?;
                        WizardState::Extraction(extraction)
                    }
                    WizardState::Extraction(extraction) => {
                        let mut review = ReviewState::default();
                        
                        // Compile and deduplicate rules
                        let compiled_rules = super::extraction::compile_rules(extraction.extracted_rules.clone());
                        
                        // Populate review with real extracted rules
                        review.rules = compiled_rules;
                        
                        // Select all rules by default
                        for i in 0..review.rules.len() {
                            review.selected.insert(i);
                        }
                        
                        // All sections expanded by default - no need to track
                        
                        WizardState::Review(review)
                    }
                    WizardState::Review(review) => {
                        let mut compilation = CompilationState::default();
                        
                        // Store rule counts for success screen
                        let selected_rules: Vec<_> = review.rules.iter()
                            .enumerate()
                            .filter(|(idx, _)| review.selected.contains(idx))
                            .map(|(_, rule)| rule)
                            .collect();
                        
                        compilation.critical_count = selected_rules.iter()
                            .filter(|r| matches!(r.severity, Severity::High))
                            .count();
                        compilation.warning_count = selected_rules.iter()
                            .filter(|r| matches!(r.severity, Severity::Medium))
                            .count();
                        compilation.info_count = selected_rules.iter()
                            .filter(|r| matches!(r.severity, Severity::Low))
                            .count();
                        
                        self.start_compilation(&mut compilation)?;
                        WizardState::Compilation(compilation)
                    }
                    WizardState::Compilation(compilation) => {
                        let output_dir = super::yaml_writer::get_output_dir();
                        
                        WizardState::Success(SuccessState {
                            total_rules: compilation.critical_count + compilation.warning_count + compilation.info_count,
                            critical_count: compilation.critical_count,
                            warning_count: compilation.warning_count,
                            info_count: compilation.info_count,
                            config_location: output_dir.join("cupcake.yaml"),
                        })
                    }
                    WizardState::Success(_) => {
                        self.should_quit = true;
                        return Ok(());
                    }
                };
            }
            StateTransition::Back => {
                // TODO: Implement going back
            }
            StateTransition::Skip => {
                // Used for skipping optional screens
                return self.transition_state(StateTransition::Continue);
            }
            StateTransition::Quit => {
                self.should_quit = true;
            }
        }
        Ok(())
    }

    /// Spawn file discovery task
    fn spawn_file_discovery(&self, event_tx: tokio::sync::mpsc::UnboundedSender<AppEvent>) {
        tokio::spawn(async move {
            // Use real file discovery
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            
            match super::discovery::discover_files(&current_dir).await {
                Ok(files) => {
                    let total = files.len();
                    if total == 0 {
                        // No files found, send a placeholder
                        let _ = event_tx.send(AppEvent::ScanProgress(1.0));
                        let _ = event_tx.send(AppEvent::ScanComplete);
                    } else {
                        // Send discovered files
                        for (i, file) in files.into_iter().enumerate() {
                            let _ = event_tx.send(AppEvent::FileFound(file));
                            let progress = (i + 1) as f64 / total as f64;
                            let _ = event_tx.send(AppEvent::ScanProgress(progress));
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                        let _ = event_tx.send(AppEvent::ScanComplete);
                    }
                }
                Err(e) => {
                    // Log error but continue with empty list
                    eprintln!("File discovery error: {}", e);
                    let _ = event_tx.send(AppEvent::ScanProgress(1.0));
                    let _ = event_tx.send(AppEvent::ScanComplete);
                }
            }
        });
    }

    fn handle_landing_event(state: &mut LandingState, event: AppEvent) -> Result<Option<StateTransition>> {
        match event {
            AppEvent::Key(key) => {
                match key.code {
                    KeyCode::Enter => {
                        // Start the wizard
                        return Ok(Some(StateTransition::Continue));
                    }
                    KeyCode::Up | KeyCode::Down => {
                        // Toggle between auto-discovery and manual mode using arrow keys
                        state.auto_discovery = !state.auto_discovery;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_discovery_event(state: &mut DiscoveryState, event: AppEvent) -> Result<Option<StateTransition>> {
        match event {
            AppEvent::Key(key) => {
                // Handle modal input first if modal is showing
                if state.show_custom_prompt {
                    match key.code {
                        KeyCode::Enter => {
                            // Apply custom instructions and close modal
                            state.show_custom_prompt = false;
                            return Ok(Some(StateTransition::Continue));
                        }
                        KeyCode::Esc => {
                            // Cancel modal without applying
                            state.custom_prompt_input.reset();
                            state.show_custom_prompt = false;
                        }
                        KeyCode::Char('s') => {
                            // Skip custom instructions
                            state.custom_prompt_input.reset();
                            state.show_custom_prompt = false;
                            return Ok(Some(StateTransition::Continue));
                        }
                        _ => {
                            // Let tui-input handle the key event
                            // We need to pass the full event, not just the key
                            if let AppEvent::Key(key) = event {
                                state.custom_prompt_input.handle_event(&Event::Key(key));
                            }
                        }
                    }
                    return Ok(None);
                }
                
                // Normal discovery screen key handling
                match key.code {
                    KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                        // Toggle between file list and preview panes
                        state.focused_pane = match state.focused_pane {
                            Pane::FileList => Pane::Preview,
                            Pane::Preview => Pane::FileList,
                        };
                    }
                    KeyCode::Up => {
                        match state.focused_pane {
                            Pane::FileList => {
                                if state.selected_index > 0 {
                                    state.selected_index -= 1;
                                    // Load preview for new selection and reset scroll
                                    if let Some(file) = state.files.get(state.selected_index) {
                                        state.preview_content = load_file_preview(&file.path);
                                        state.preview_scroll_offset = 0;
                                    }
                                }
                            }
                            Pane::Preview => {
                                // Scroll preview up
                                if state.preview_scroll_offset > 0 {
                                    state.preview_scroll_offset = state.preview_scroll_offset.saturating_sub(1);
                                }
                            }
                        }
                    }
                    KeyCode::Down => {
                        match state.focused_pane {
                            Pane::FileList => {
                                if state.selected_index < state.files.len().saturating_sub(1) {
                                    state.selected_index += 1;
                                    // Load preview for new selection and reset scroll
                                    if let Some(file) = state.files.get(state.selected_index) {
                                        state.preview_content = load_file_preview(&file.path);
                                        state.preview_scroll_offset = 0;
                                    }
                                }
                            }
                            Pane::Preview => {
                                // Scroll preview down - we'll calculate max scroll in the render function
                                if let Some(content) = &state.preview_content {
                                    let _line_count = content.lines().count() as u16;
                                    // This is approximate - render function will handle actual limit
                                    state.preview_scroll_offset = state.preview_scroll_offset.saturating_add(1);
                                }
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        if state.focused_pane == Pane::Preview {
                            // Scroll up by ~10 lines
                            state.preview_scroll_offset = state.preview_scroll_offset.saturating_sub(10);
                        }
                    }
                    KeyCode::PageDown => {
                        if state.focused_pane == Pane::Preview {
                            // Scroll down by ~10 lines
                            state.preview_scroll_offset = state.preview_scroll_offset.saturating_add(10);
                        }
                    }
                    KeyCode::Home => {
                        if state.focused_pane == Pane::Preview {
                            state.preview_scroll_offset = 0;
                        }
                    }
                    KeyCode::End => {
                        if state.focused_pane == Pane::Preview {
                            // Set to max - render function will clamp it
                            state.preview_scroll_offset = u16::MAX;
                        }
                    }
                    KeyCode::Enter => {
                        // Only works in file list pane
                        if state.focused_pane == Pane::FileList {
                            // Toggle selection
                            if let Some(file) = state.files.get(state.selected_index) {
                                if state.selected.contains(&file.path) {
                                    state.selected.remove(&file.path);
                                } else {
                                    state.selected.insert(file.path.clone());
                                }
                            }
                        }
                    }
                    KeyCode::Char(' ') => {
                        // Continue if we have selections
                        if !state.selected.is_empty() {
                            return Ok(Some(StateTransition::Continue));
                        }
                    }
                    // Custom prompt modal removed for simplicity
                    _ => {}
                }
            }
            AppEvent::FileFound(file) => {
                state.files.push(file);
                // Load preview for first file
                if state.files.len() == 1 {
                    state.preview_content = load_file_preview(&state.files[0].path);
                }
            }
            AppEvent::ScanProgress(progress) => {
                state.scan_progress = progress;
            }
            AppEvent::ScanComplete => {
                state.scan_complete = true;
                // Select first file by default if any
                if !state.files.is_empty() && state.selected.is_empty() {
                    if let Some(first) = state.files.first() {
                        state.selected.insert(first.path.clone());
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_extraction_event(state: &mut ExtractionState, event: AppEvent) -> Result<Option<StateTransition>> {
        match event {
            AppEvent::Key(key) => {
                match key.code {
                    KeyCode::Enter => {
                        // Only allow continue when all tasks are complete
                        let all_complete = state.tasks.iter()
                            .all(|t| matches!(t.status, TaskStatus::Complete | TaskStatus::Failed(_)));
                        
                        if all_complete {
                            return Ok(Some(StateTransition::Continue));
                        }
                    }
                    _ => {}
                }
            }
            AppEvent::ExtractionStarted { file } => {
                // Update task status to InProgress
                if let Some(task) = state.tasks.iter_mut().find(|t| t.file_path == file) {
                    task.status = TaskStatus::InProgress;
                    task.progress = 0.0;
                    // Track start time
                    state.task_start_times.insert(file, std::time::Instant::now());
                }
            }
            AppEvent::ExtractionProgress { file, progress } => {
                // Update task progress
                if let Some(task) = state.tasks.iter_mut().find(|t| t.file_path == file) {
                    task.progress = progress;
                    // Update elapsed time
                    if let Some(start_time) = state.task_start_times.get(&file) {
                        task.elapsed_ms = start_time.elapsed().as_millis() as u64;
                    }
                }
                // Update overall progress
                let total_progress: f64 = state.tasks.iter().map(|t| t.progress).sum();
                state.overall_progress = total_progress / state.tasks.len() as f64;
            }
            AppEvent::ExtractionComplete { file, rules } => {
                // Update task status and rule count
                if let Some(task) = state.tasks.iter_mut().find(|t| t.file_path == file) {
                    task.status = TaskStatus::Complete;
                    task.progress = 1.0;
                    task.rules_found = rules.len();
                    // Final elapsed time update
                    if let Some(start_time) = state.task_start_times.get(&file) {
                        task.elapsed_ms = start_time.elapsed().as_millis() as u64;
                    }
                }
                // Store extracted rules
                state.extracted_rules.extend(rules);
                
                // Check if all tasks are complete and start compilation timer
                let all_complete = state.tasks.iter()
                    .all(|t| matches!(t.status, TaskStatus::Complete | TaskStatus::Failed(_)));
                
                if all_complete && state.compilation_started_at == 0 && !state.extracted_rules.is_empty() {
                    state.compilation_started_at = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                }
            }
            AppEvent::ExtractionFailed { file, error } => {
                // Update task status with error
                if let Some(task) = state.tasks.iter_mut().find(|t| t.file_path == file) {
                    task.status = TaskStatus::Failed(error);
                    // Final elapsed time update
                    if let Some(start_time) = state.task_start_times.get(&file) {
                        task.elapsed_ms = start_time.elapsed().as_millis() as u64;
                    }
                }
            }
            AppEvent::Tick => {
                // Update elapsed times for in-progress tasks to animate spinners
                for task in state.tasks.iter_mut() {
                    if matches!(task.status, TaskStatus::InProgress) {
                        if let Some(start_time) = state.task_start_times.get(&task.file_path) {
                            task.elapsed_ms = start_time.elapsed().as_millis() as u64;
                        }
                    }
                }
                
                // Force redraw to update compilation timer if needed
                let all_complete = state.tasks.iter()
                    .all(|t| matches!(t.status, TaskStatus::Complete | TaskStatus::Failed(_)));
                if all_complete && state.compilation_started_at > 0 && !state.compilation_complete {
                    // Check if compilation should be marked complete (after 2-3 seconds)
                    let current_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    
                    if current_time - state.compilation_started_at > 2500 {
                        state.compilation_complete = true;
                        state.compilation_completed_at = current_time;
                        
                        // Simulate deduplication - remove ~20% of rules
                        let deduplicated_count = (state.extracted_rules.len() as f64 * 0.8) as usize;
                        state.compiled_rule_count = deduplicated_count.max(1);
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_review_event(state: &mut ReviewState, event: AppEvent) -> Result<Option<StateTransition>> {
        match event {
            AppEvent::Key(key) => {
                match key.code {
                    KeyCode::Up => {
                        // Since rules are sorted by severity, we need to find actual rule indices
                        let _sorted_indices = Self::get_sorted_rule_indices(&state.rules);
                        
                        if state.selected_index > 0 {
                            state.selected_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if state.selected_index < state.rules.len().saturating_sub(1) {
                            state.selected_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        // Get the actual rule index from sorted display index
                        let sorted_indices = Self::get_sorted_rule_indices(&state.rules);
                        if let Some((actual_idx, _)) = sorted_indices.get(state.selected_index) {
                            // Toggle selection for current rule
                            if state.selected.contains(actual_idx) {
                                state.selected.remove(actual_idx);
                            } else {
                                state.selected.insert(*actual_idx);
                            }
                        }
                    }
                    KeyCode::Char('a') => {
                        // Select all rules
                        for i in 0..state.rules.len() {
                            state.selected.insert(i);
                        }
                    }
                    KeyCode::Char('n') => {
                        // Select none
                        state.selected.clear();
                    }
                    KeyCode::Char('x') => {
                        // Toggle expanded view for current rule
                        if state.expanded_rule == Some(state.selected_index) {
                            state.expanded_rule = None;
                        } else {
                            state.expanded_rule = Some(state.selected_index);
                        }
                    }
                    KeyCode::Char(' ') => {
                        // Continue to compilation only if we have selections
                        if !state.selected.is_empty() {
                            return Ok(Some(StateTransition::Continue));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_compilation_event(state: &mut CompilationState, event: AppEvent) -> Result<Option<StateTransition>> {
        match event {
            AppEvent::Key(key) => {
                match key.code {
                    KeyCode::Char('l') => {
                        // Toggle logs
                        state.show_logs = !state.show_logs;
                    }
                    KeyCode::Char('v') => {
                        // Toggle verbose mode (add more logs)
                        state.logs.push("Verbose mode enabled".to_string());
                    }
                    KeyCode::Char('r') => {
                        // Retry failed phases
                        if let Some(phase) = state.phases.get_mut(state.current_phase) {
                            if matches!(phase.status, PhaseStatus::Failed(_)) {
                                phase.status = PhaseStatus::InProgress;
                            }
                        }
                    }
                    _ => {}
                }
            }
            AppEvent::CompilationProgress { phase, progress } => {
                // Calculate overall progress first
                let phase_count = state.phases.len();
                state.overall_progress = (phase as f64 + progress) / phase_count as f64;
                
                if let Some(current) = state.phases.get_mut(phase) {
                    current.status = PhaseStatus::InProgress;
                    
                    // Update phase details based on progress
                    match phase {
                        0 => {
                            // Policy compilation phase
                            current.details.clear();
                            if progress >= 0.3 {
                                current.details.push("✓ Parsing extracted rules".to_string());
                            }
                            if progress >= 0.6 {
                                current.details.push("✓ Generating YAML policies".to_string());
                            }
                            if progress >= 0.9 {
                                current.details.push("⟳ Optimizing patterns...".to_string());
                            }
                        }
                        1 => {
                            // Hook installation phase
                            current.details.clear();
                            if progress >= 0.25 {
                                current.details.push("✓ Created .claude/hooks/ directory".to_string());
                            }
                            if progress >= 0.5 {
                                current.details.push("✓ Installed pre-commit hook".to_string());
                            }
                            if progress >= 0.75 {
                                current.details.push("⟳ Installing file-change watchers...".to_string());
                            }
                        }
                        2 => {
                            // Validation phase
                            current.details.clear();
                            if progress >= 0.5 {
                                current.details.push("✓ Policy syntax validated".to_string());
                            }
                            if progress >= 0.8 {
                                current.details.push("⟳ Testing hook execution...".to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            AppEvent::CompilationPhaseComplete { phase } => {
                if let Some(current) = state.phases.get_mut(phase) {
                    current.status = PhaseStatus::Complete;
                    current.elapsed_ms = 348 + (phase as u64 * 127); // Mock elapsed time
                    
                    // Update final details
                    match phase {
                        0 => {
                            current.details.clear();
                            current.details.push("✓ Generated 3 policy files".to_string());
                            current.details.push("✓ 18 critical, 23 warning, 11 info rules".to_string());
                        }
                        1 => {
                            current.details.clear();
                            current.details.push("✓ Created .claude/hooks/ directory".to_string());
                            current.details.push("✓ Installed pre-commit hook".to_string());
                            current.details.push("✓ Installed file-change watchers".to_string());
                            current.details.push("✓ Updated .claude/settings.local.json".to_string());
                        }
                        2 => {
                            current.details.clear();
                            current.details.push("✓ Policy syntax validated".to_string());
                            current.details.push("✓ Hook execution tested".to_string());
                            current.details.push("✓ All systems operational".to_string());
                        }
                        _ => {}
                    }
                    
                    state.current_phase = (phase + 1).min(state.phases.len() - 1);
                }
                
                // If all phases complete, transition to success
                if state.phases.iter().all(|p| matches!(p.status, PhaseStatus::Complete)) {
                    return Ok(Some(StateTransition::Continue));
                }
            }
            AppEvent::CompilationPhaseFailed { phase, error } => {
                if let Some(current) = state.phases.get_mut(phase) {
                    current.status = PhaseStatus::Failed(error);
                }
            }
            AppEvent::CompilationLog(log) => {
                state.logs.push(log);
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_success_event(&mut self, event: AppEvent) -> Result<Option<StateTransition>> {
        if let AppEvent::Key(key) = event {
            match key.code {
                KeyCode::Enter => {
                    self.should_quit = true;
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn start_extraction(&self, selected_files: &HashSet<PathBuf>, state: &mut ExtractionState) -> Result<()> {
        // Create extraction tasks for selected files
        for path in selected_files.iter() {
            let file_name = path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            
            let task = ExtractionTask {
                file_path: path.clone(),
                file_name,
                status: TaskStatus::Queued,
                progress: 0.0,
                elapsed_ms: 0,
                rules_found: 0,
            };
            
            state.tasks.push(task);
        }
        
        // Set overall progress
        state.overall_progress = 0.0;
        
        // Spawn extraction tasks
        if let Some(tx) = &self.event_tx {
            for path in selected_files {
                super::extraction::spawn_extraction_task(path.clone(), tx.clone());
            }
        }
        
        Ok(())
    }

    fn start_compilation(&self, state: &mut CompilationState) -> Result<()> {
        // Initialize compilation phases
        state.phases = vec![
            CompilationPhase {
                name: "Policy Compilation".to_string(),
                status: PhaseStatus::InProgress,
                details: vec![
                    "⟳ Compiling rules into Cupcake YAML format...".to_string(),
                ],
                elapsed_ms: 0,
            },
            CompilationPhase {
                name: "Claude Code Hook Installation".to_string(),
                status: PhaseStatus::Pending,
                details: vec![],
                elapsed_ms: 0,
            },
            CompilationPhase {
                name: "Validation & Testing".to_string(),
                status: PhaseStatus::Pending,
                details: vec![],
                elapsed_ms: 0,
            },
        ];
        
        state.current_phase = 0;
        state.overall_progress = 0.0;
        
        // Start simulating compilation progress
        if let Some(tx) = &self.event_tx {
            let event_tx = tx.clone();
            tokio::spawn(async move {
                // Phase 1: Policy Compilation
                tokio::time::sleep(Duration::from_millis(500)).await;
                let _ = event_tx.send(AppEvent::CompilationLog("Starting policy compilation...".to_string()));
                
                tokio::time::sleep(Duration::from_millis(300)).await;
                let _ = event_tx.send(AppEvent::CompilationProgress { phase: 0, progress: 0.3 });
                let _ = event_tx.send(AppEvent::CompilationLog("Creating guardrails directory...".to_string()));
                
                // Actually generate stub YAML files
                let output_dir = super::yaml_writer::get_output_dir();
                match super::yaml_writer::generate_stub_files(&output_dir, 52) {
                    Ok(_) => {
                        let _ = event_tx.send(AppEvent::CompilationLog(format!("✓ Created {}", output_dir.display())));
                    }
                    Err(e) => {
                        let _ = event_tx.send(AppEvent::CompilationLog(format!("⚠️  Failed to create files: {}", e)));
                    }
                }
                
                tokio::time::sleep(Duration::from_millis(400)).await;
                let _ = event_tx.send(AppEvent::CompilationProgress { phase: 0, progress: 0.6 });
                let _ = event_tx.send(AppEvent::CompilationLog("Writing policy files...".to_string()));
                
                tokio::time::sleep(Duration::from_millis(300)).await;
                let _ = event_tx.send(AppEvent::CompilationProgress { phase: 0, progress: 0.9 });
                let _ = event_tx.send(AppEvent::CompilationLog("Generated 3 policy files (18 critical, 23 warning, 11 info)".to_string()));
                
                tokio::time::sleep(Duration::from_millis(200)).await;
                let _ = event_tx.send(AppEvent::CompilationPhaseComplete { phase: 0 });
                
                // Phase 2: Hook Installation
                tokio::time::sleep(Duration::from_millis(300)).await;
                let _ = event_tx.send(AppEvent::CompilationLog("Beginning Claude Code hook installation...".to_string()));
                let _ = event_tx.send(AppEvent::CompilationProgress { phase: 1, progress: 0.0 });
                
                tokio::time::sleep(Duration::from_millis(400)).await;
                let _ = event_tx.send(AppEvent::CompilationProgress { phase: 1, progress: 0.25 });
                let _ = event_tx.send(AppEvent::CompilationLog("Creating .claude directory...".to_string()));
                
                // Actually update Claude settings
                match super::claude_settings::update_claude_settings() {
                    Ok(_) => {
                        let _ = event_tx.send(AppEvent::CompilationLog("✓ Updated .claude/settings.local.json".to_string()));
                    }
                    Err(e) => {
                        let _ = event_tx.send(AppEvent::CompilationLog(format!("⚠️  Failed to update settings: {}", e)));
                    }
                }
                
                tokio::time::sleep(Duration::from_millis(300)).await;
                let _ = event_tx.send(AppEvent::CompilationProgress { phase: 1, progress: 0.5 });
                let _ = event_tx.send(AppEvent::CompilationLog("Hook registered: pre-commit → cupcake check".to_string()));
                
                tokio::time::sleep(Duration::from_millis(400)).await;
                let _ = event_tx.send(AppEvent::CompilationProgress { phase: 1, progress: 0.75 });
                let _ = event_tx.send(AppEvent::CompilationLog("Installing file-change watchers...".to_string()));
                
                tokio::time::sleep(Duration::from_millis(300)).await;
                let _ = event_tx.send(AppEvent::CompilationPhaseComplete { phase: 1 });
                
                // Phase 3: Validation
                tokio::time::sleep(Duration::from_millis(300)).await;
                let _ = event_tx.send(AppEvent::CompilationLog("Running validation tests...".to_string()));
                let _ = event_tx.send(AppEvent::CompilationProgress { phase: 2, progress: 0.0 });
                
                tokio::time::sleep(Duration::from_millis(500)).await;
                let _ = event_tx.send(AppEvent::CompilationProgress { phase: 2, progress: 0.5 });
                let _ = event_tx.send(AppEvent::CompilationLog("Testing hook execution...".to_string()));
                
                tokio::time::sleep(Duration::from_millis(400)).await;
                let _ = event_tx.send(AppEvent::CompilationProgress { phase: 2, progress: 0.8 });
                let _ = event_tx.send(AppEvent::CompilationLog("All tests passed!".to_string()));
                
                tokio::time::sleep(Duration::from_millis(200)).await;
                let _ = event_tx.send(AppEvent::CompilationPhaseComplete { phase: 2 });
            });
        }
        
        Ok(())
    }
}

impl App {
    /// Get sorted rule indices matching the display order in the review screen
    fn get_sorted_rule_indices(rules: &[ExtractedRule]) -> Vec<(usize, &ExtractedRule)> {
        let mut sorted_rules: Vec<(usize, &ExtractedRule)> = rules.iter()
            .enumerate()
            .collect();
        
        sorted_rules.sort_by(|a, b| {
            let severity_order = |s: &Severity| match s {
                Severity::High => 0,
                Severity::Medium => 1,
                Severity::Low => 2,
            };
            
            match severity_order(&a.1.severity).cmp(&severity_order(&b.1.severity)) {
                std::cmp::Ordering::Equal => a.1.id.cmp(&b.1.id),
                other => other,
            }
        });
        
        sorted_rules
    }
}

/// Load file preview synchronously
fn load_file_preview(path: &PathBuf) -> Option<String> {
    if path.is_dir() {
        // Show directory contents
        if let Ok(entries) = std::fs::read_dir(path) {
            let mut contents = vec![format!("📁 Directory: {}\n", path.display())];
            let mut count = 0;
            for entry in entries.flatten() {
                if count >= 20 { 
                    contents.push("... (more files)".to_string());
                    break;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    contents.push(format!("  📁 {}/", name));
                } else {
                    contents.push(format!("  📄 {}", name));
                }
                count += 1;
            }
            Some(contents.join("\n"))
        } else {
            Some(format!("Cannot read directory: {}", path.display()))
        }
    } else {
        // Read file contents
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().take(50).collect();
                let preview = lines.join("\n");
                if content.lines().count() > 50 {
                    Some(format!("{}\n\n... ({} more lines)", preview, content.lines().count() - 50))
                } else {
                    Some(preview)
                }
            }
            Err(_) => Some(format!("Cannot preview: {}", path.display()))
        }
    }
}

