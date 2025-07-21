**this is not a perfect guide**

**this is only meant to serve as a reference**

# Cupcake Init TUI - Final Implementation Outline

## Project Structure

```
src/cli/tui/
├── mod.rs              # Public API - exports run_init_wizard()
├── init/
│   ├── mod.rs          # Init wizard entry point
│   ├── app.rs          # Main App struct and event loop
│   ├── state.rs        # State machine and screen states
│   ├── events.rs       # Event types and routing
│   ├── screens/
│   │   ├── mod.rs      # Screen trait definition
│   │   ├── discovery.rs    # Screen 1: File selection with preview
│   │   ├── extraction.rs   # Screen 3: Parallel rule extraction
│   │   ├── review.rs       # Screen 4: Rule review with search
│   │   └── compilation.rs  # Screen 5: Compilation progress
│   ├── components/     # Reusable UI components
│   │   ├── mod.rs
│   │   ├── file_list.rs    # File tree with checkboxes
│   │   ├── modal.rs        # Modal overlay helper
│   │   ├── progress_table.rs # Table with embedded gauges
│   │   └── dropdown.rs     # Simple dropdown widget
│   └── theme.rs        # Consistent colors and styles
```

## Core Types

```rust
// src/cli/tui/init/state.rs

/// Main state machine for the wizard
pub enum WizardState {
    Discovery(DiscoveryState),
    Extraction(ExtractionState),
    Review(ReviewState),
    Compilation(CompilationState),
    Success(SuccessState),
}

pub struct DiscoveryState {
    // File discovery
    files: Vec<RuleFile>,
    selected: HashSet<PathBuf>,
    scan_complete: bool,
    scan_progress: f64,

    // UI state
    list_state: ListState,
    focused_pane: Pane,
    preview_content: Option<String>,

    // Custom prompt modal
    show_custom_prompt: bool,
    custom_prompt_input: Input,
}

pub struct RuleFile {
    path: PathBuf,
    agent: Agent,
    is_directory: bool,
    children: Vec<PathBuf>, // for directories
}

#[derive(Clone, Copy)]
pub enum Agent {
    Claude,
    Cursor,
    Windsurf,
    Kiro,
    // ... etc
}
```

## Event System

```rust
// src/cli/tui/init/events.rs

pub enum AppEvent {
    // Input events
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),

    // Async task events
    FileFound(RuleFile),
    ScanProgress(f64),
    ScanComplete,

    ExtractionStarted { file: PathBuf },
    ExtractionProgress { file: PathBuf, progress: f64 },
    ExtractionComplete { file: PathBuf, rules: Vec<Rule> },
    ExtractionFailed { file: PathBuf, error: String },

    CompilationProgress { phase: Phase, progress: f64 },
    CompilationLog(LogEntry),

    // UI events
    Tick, // for smooth progress bars
}

/// Route events to current screen
impl App {
    fn handle_event(&mut self, event: AppEvent) -> Result<Option<StateTransition>> {
        match &mut self.state {
            WizardState::Discovery(state) => self.handle_discovery_event(state, event),
            WizardState::Extraction(state) => self.handle_extraction_event(state, event),
            // ... etc
        }
    }
}
```

## Screen Implementations

### Screen 1: Discovery with Preview

```rust
// src/cli/tui/init/screens/discovery.rs

impl DiscoveryScreen {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        // Split into file list and preview
        let chunks = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ]).split(area);

        // Left: File list with checkboxes
        self.render_file_list(f, chunks[0]);

        // Right: Preview
        self.render_preview(f, chunks[1]);

        // Modal: Custom prompt (if active)
        if self.state.show_custom_prompt {
            self.render_custom_prompt_modal(f, f.area());
        }
    }

    fn render_file_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.state.files.iter().map(|file| {
            let checkbox = if self.state.selected.contains(&file.path) { "☑" } else { "☐" };
            let icon = if file.is_directory { "📁" } else { "" };
            let badge = format!("[{}]", file.agent);

            ListItem::new(Line::from(vec![
                Span::raw(format!("{} ", checkbox)),
                Span::raw(file.path.display().to_string()),
                Span::raw(" "),
                Span::styled(badge, Style::default().fg(Color::Cyan)),
            ]))
        }).collect();

        let list = List::new(items)
            .block(Block::bordered().title("Select Rule Sources"))
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(list, area, &mut self.state.list_state);
    }
}
```

### Screen 2: Custom Prompt Modal

```rust
// src/cli/tui/init/components/modal.rs

pub fn render_input_modal(
    f: &mut Frame,
    area: Rect,
    title: &str,
    input: &Input,
    is_active: bool,
) {
    let block = Block::bordered()
        .title(title)
        .border_style(if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    let inner = block.inner(area);
    f.render_widget(Clear, area); // Clear background
    f.render_widget(block, area);

    // Render input with scroll support
    let width = inner.width.max(3) - 3;
    let scroll = input.visual_scroll(width as usize);

    let input_widget = Paragraph::new(input.value())
        .scroll((0, scroll as u16));

    f.render_widget(input_widget, inner);

    // Set cursor position when active
    if is_active {
        let x = input.visual_cursor().max(scroll) - scroll + 1;
        f.set_cursor_position((inner.x + x as u16, inner.y));
    }
}
```

### Screen 3: Parallel Extraction

```rust
// src/cli/tui/init/screens/extraction.rs

impl ExtractionScreen {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let header_rows = vec![
            Row::new(vec!["File", "Progress", "Status", "Time", "Rules"]),
        ];

        let data_rows: Vec<Row> = self.state.tasks.iter().map(|task| {
            let progress_gauge = Gauge::default()
                .percent(task.progress as u16)
                .gauge_style(Style::default().fg(Color::Green))
                .label("");

            Row::new(vec![
                Cell::from(task.file_name.as_str()),
                Cell::from(progress_gauge),
                Cell::from(self.status_icon(&task.status)),
                Cell::from(format!("{}ms", task.elapsed_ms)),
                Cell::from(task.rules_found.to_string()),
            ])
        }).collect();

        let table = Table::new(data_rows, [
            Constraint::Percentage(30),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(6),
        ])
        .header(Row::new(header_rows[0].clone()).style(Style::default().bold()))
        .block(Block::bordered().title("Extracting Rules"));

        f.render_widget(table, area);
    }
}
```

### Screen 4: Rule Review with Search

```rust
// src/cli/tui/init/screens/review.rs

pub struct ReviewState {
    rules: Vec<ExtractedRule>,
    selected: HashSet<usize>,
    filtered_indices: Vec<usize>,

    // UI state
    list_state: ListState,
    search_input: Input,
    search_active: bool,

    // Edit modal
    editing_rule: Option<usize>,
    edit_form: RuleEditForm,
}

struct RuleEditForm {
    description: Input,
    severity: Severity,
    category: Category,
    current_field: FormField,
}

impl ReviewScreen {
    fn handle_key(&mut self, key: KeyCode) -> Option<StateTransition> {
        if self.state.search_active {
            match key {
                KeyCode::Esc => {
                    self.state.search_active = false;
                    self.state.search_input.reset();
                    self.update_filter();
                }
                KeyCode::Enter => self.state.search_active = false,
                _ => {
                    // Let tui-input handle the key
                    // (handled in parent)
                    self.update_filter();
                }
            }
        } else if let Some(_) = self.state.editing_rule {
            self.handle_edit_keys(key)
        } else {
            match key {
                KeyCode::Char('/') => self.state.search_active = true,
                KeyCode::Char(' ') => self.toggle_selected(),
                KeyCode::Char('e') => self.start_editing(),
                KeyCode::Enter => return Some(StateTransition::Continue),
                _ => {}
            }
        }
        None
    }
}
```

### Screen 5: Compilation Progress

```rust
// src/cli/tui/init/screens/compilation.rs

impl CompilationScreen {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(3),  // Overall progress
            Constraint::Length(10), // Phase details
            Constraint::Min(5),     // Logs
        ]).split(area);

        // Overall progress bar
        let overall = Gauge::default()
            .block(Block::bordered())
            .gauge_style(Style::default().fg(Color::Blue))
            .percent(self.state.overall_progress)
            .label(format!("Phase {} of 3", self.state.current_phase));

        f.render_widget(overall, chunks[0]);

        // Phase breakdown
        self.render_phases(f, chunks[1]);

        // Log viewer (collapsible)
        if self.state.show_logs {
            self.render_logs(f, chunks[2]);
        }
    }
}
```

## Main Event Loop

```rust
// src/cli/tui/init/app.rs

impl App {
    pub async fn run(mut self) -> Result<()> {
        let mut terminal = ratatui::init();

        // Spawn async tasks
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        self.spawn_background_tasks(event_tx.clone());

        // Spawn input handler
        let input_tx = event_tx.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(event) = event::read() {
                    match event {
                        Event::Key(key) => {
                            let _ = input_tx.send(AppEvent::Key(key));
                        }
                        Event::Resize(w, h) => {
                            let _ = input_tx.send(AppEvent::Resize(w, h));
                        }
                        _ => {}
                    }
                }
            }
        });

        // Main render loop
        loop {
            terminal.draw(|f| self.render(f))?;

            // Handle events with timeout for animations
            match tokio::time::timeout(
                Duration::from_millis(50),
                event_rx.recv()
            ).await {
                Ok(Some(event)) => {
                    if let Some(transition) = self.handle_event(event)? {
                        self.transition_state(transition)?;
                    }
                }
                Ok(None) => break, // Channel closed
                Err(_) => {
                    // Timeout - send tick for animations
                    self.handle_event(AppEvent::Tick)?;
                }
            }

            if self.should_quit {
                break;
            }
        }

        ratatui::restore();
        Ok(())
    }
}
```

## Key Implementation Details

1. **Text Input Handling**: All text inputs use `tui-input`:

   ```rust
   KeyEvent(key) if self.has_active_input() => {
       self.active_input_mut().handle_event(&Event::Key(key));
   }
   ```

2. **Async Task Management**: Background tasks report via channels:

   ```rust
   tokio::spawn(async move {
       for file in discover_files(root).await {
           let _ = tx.send(AppEvent::FileFound(file));
       }
       let _ = tx.send(AppEvent::ScanComplete);
   });
   ```

3. **State Transitions**: Clean, type-safe transitions:

   ```rust
   enum StateTransition {
       Continue,      // Go to next screen
       Back,          // Go to previous screen
       Skip,          // Skip optional screen
       Quit,          // Exit wizard
   }
   ```

4. **Theme Consistency**: Centralized styling:
   ```rust
   pub struct Theme {
       pub selected: Style,
       pub error: Style,
       pub success: Style,
       pub critical: Style,
       pub warning: Style,
       pub info: Style,
   }
   ```

This implementation provides a smooth, responsive TUI experience using only ratatui's built-in widgets plus `tui-input` for text handling. No custom cursor tracking or complex widget implementations needed!

**this is not a perfect guide**

**this is only meant to serve as a reference**
