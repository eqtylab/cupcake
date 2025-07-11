# Plan 001: Core Domain and Type-Safe Foundation - Implementation Plan

Status: Ready for Implementation
Dependencies: Design Phase Documentation
Enables: Plans 002, 003, 004

## Overview

This plan implements the foundational Rust data structures and project scaffolding for Cupcake MVP. The implementation follows a phased approach that enables incremental validation and testing while maintaining strict alignment with the design documentation.

**Implementation Note**: This plan is divided into 6 phases that should be executed sequentially. Each phase builds upon the previous and includes specific validation steps to ensure correctness before proceeding.

## Core Principles

- **Design Alignment**: Every structure must exactly match the specifications in `context/design_phase/`
- **Type Safety**: Leverage Rust's type system for correctness guarantees
- **Testability**: Each phase produces independently testable components
- **Simplicity**: Focus on elegant, minimal implementations that satisfy requirements

## Dependencies and Versions

Based on `context/lib_docs/`, use these exact versions:

```toml
[dependencies]
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.140"
toml = "0.9.1"
clap = { version = "4.5.41", features = ["derive"] }
anyhow = "1.0.98"
thiserror = "2.0.12"
regex = "1.11.1"
glob = "0.3.2"
bincode = "2.0.1"
directories = "6.0.0"
walkdir = "2.5.0"

[dev-dependencies]
# For testing serialization/deserialization
pretty_assertions = "1.4"
```

## Phase 1: Project Scaffolding and Core Types

### Objectives
- Create project structure with proper module organization
- Implement core error types
- Set up basic type definitions

### Implementation Steps

1. **Initialize Cargo Project**
   ```bash
   cargo init --name cupcake
   ```

2. **Create Module Structure**
   ```
   src/
   ├── main.rs
   ├── lib.rs
   ├── cli/
   │   └── mod.rs
   ├── engine/
   │   └── mod.rs
   ├── config/
   │   └── mod.rs
   ├── state/
   │   └── mod.rs
   └── io/
       └── mod.rs
   ```

3. **Implement Core Error Types** (`src/error.rs`)
   ```rust
   use thiserror::Error;
   
   #[derive(Error, Debug)]
   pub enum CupcakeError {
       #[error("Policy parsing error: {0}")]
       PolicyParse(String),
       #[error("IO error: {0}")]
       Io(#[from] std::io::Error),
       #[error("Serialization error: {0}")]
       Serialization(#[from] toml::de::Error),
       // Add other variants as needed
   }
   
   pub type Result<T> = std::result::Result<T, CupcakeError>;
   ```

### Validation
- Project compiles with `cargo build`
- All modules are accessible from main
- Error types can be instantiated and displayed

## Phase 2: Policy Schema Implementation

### Objectives
- Implement all policy-related data structures from `policy-schema.md`
- Ensure perfect serde serialization/deserialization with TOML

### Implementation Steps

1. **Core Policy Types** (`src/config/types.rs`)
   ```rust
   use serde::{Deserialize, Serialize};
   use std::collections::HashMap;
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct PolicyFile {
       pub schema_version: String,
       #[serde(default)]
       pub settings: Settings,
       pub policies: Vec<Policy>,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize, Default)]
   pub struct Settings {
       #[serde(default)]
       pub audit_logging: bool,
       #[serde(default)]
       pub debug_mode: bool,
   }
   ```

2. **Condition Types** (`src/config/conditions.rs`)
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[serde(tag = "type", rename_all = "snake_case")]
   pub enum Condition {
       CommandRegex { pattern: String },
       FilepathRegex { pattern: String },
       FilepathGlob { pattern: String },
       FileContentRegex { pattern: String },
       Not { condition: Box<Condition> },
       And { conditions: Vec<Condition> },
       Or { conditions: Vec<Condition> },
       StateExists { key: String },
       StateMissing { key: String },
       StateQuery { key: String, value_regex: String },
       // Additional conditions per schema
   }
   ```

3. **Action Types** (`src/config/actions.rs`)
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[serde(tag = "type", rename_all = "snake_case")]
   pub enum Action {
       ProvideFeedback { 
           message: String,
           #[serde(default)]
           include_context: bool,
       },
       BlockWithFeedback { 
           message: String,
           #[serde(default)]
           include_context: bool,
       },
       Approve,
       RunCommand {
           command: String,
           #[serde(default)]
           on_failure: OnFailureBehavior,
       },
       UpdateState {
           key: String,
           value: serde_json::Value,
       },
       Conditional {
           if_condition: Condition,
           then_action: Box<Action>,
           #[serde(skip_serializing_if = "Option::is_none")]
           else_action: Option<Box<Action>>,
       },
   }
   ```

### Tests
- Unit tests for each type's serialization/deserialization
- Round-trip tests with sample TOML files
- Validation of all enum variants

## Phase 3: Hook Event Types

### Objectives
- Implement all Claude Code hook event structures
- Ensure compatibility with actual hook payloads

### Implementation Steps

1. **Base Event Types** (`src/engine/events.rs`)
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct CommonEventData {
       pub session_id: String,
       pub transcript_path: String,
       pub hook_event_name: String,
   }
   
   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[serde(tag = "hook_event_name", rename_all = "snake_case")]
   pub enum HookEvent {
       PreToolUse {
           #[serde(flatten)]
           common: CommonEventData,
           tool_name: String,
           tool_input: serde_json::Value,
       },
       PostToolUse {
           #[serde(flatten)]
           common: CommonEventData,
           tool_name: String,
           tool_input: serde_json::Value,
           tool_response: String,
       },
       // Other variants per hook-events.md
   }
   ```

### Tests
- Deserialize sample hook payloads
- Verify field extraction works correctly
- Test template variable resolution

## Phase 4: CLI Interface Structure

### Objectives
- Implement complete CLI structure using clap
- Create empty command handlers
- Set up argument parsing

### Implementation Steps

1. **CLI App Definition** (`src/cli/app.rs`)
   ```rust
   use clap::{Parser, Subcommand};
   
   #[derive(Parser)]
   #[command(name = "cupcake")]
   #[command(about = "Deterministic policy enforcement for Claude Code")]
   pub struct Cli {
       #[command(subcommand)]
       pub command: Commands,
   }
   
   #[derive(Subcommand)]
   pub enum Commands {
       /// Interactive policy generation from CLAUDE.md files
       Init {
           #[arg(short, long)]
           output: Option<String>,
       },
       /// Runtime enforcement (called by hooks)
       Run {
           hook_event: String,
       },
       /// Updates Claude Code hooks in settings.json
       Sync {
           #[arg(short, long)]
           settings_path: Option<String>,
       },
       /// Validates policy file syntax
       Validate {
           #[arg(default_value = "cupcake.toml")]
           policy_file: String,
       },
       /// Views audit logs
       Audit {
           #[arg(short, long)]
           tail: Option<usize>,
       },
   }
   ```

2. **Command Handlers** (`src/cli/commands/`)
   - Create empty handler functions for each command
   - Wire up basic argument handling
   - Add placeholder output

### Validation
- All commands parse arguments correctly
- Help text displays properly
- Each command can be invoked without crashing

## Phase 5: Core Infrastructure

### Objectives
- Implement path utilities and constants
- Set up configuration loading framework
- Create state management structures

### Implementation Steps

1. **Path Management** (`src/io/paths.rs`)
   ```rust
   use directories::ProjectDirs;
   use std::path::PathBuf;
   
   pub struct CupcakePaths {
       pub config_dir: PathBuf,
       pub state_dir: PathBuf,
       pub cache_dir: PathBuf,
   }
   
   impl CupcakePaths {
       pub fn new() -> Result<Self> {
           let dirs = ProjectDirs::from("", "", "cupcake")
               .ok_or_else(|| anyhow!("Failed to determine project directories"))?;
           
           Ok(Self {
               config_dir: dirs.config_dir().to_path_buf(),
               state_dir: dirs.data_dir().join("state"),
               cache_dir: dirs.cache_dir().to_path_buf(),
           })
       }
   }
   ```

2. **Configuration Loading** (`src/config/loader.rs`)
   ```rust
   pub fn load_policy_file(path: &Path) -> Result<PolicyFile> {
       let contents = std::fs::read_to_string(path)?;
       let policy: PolicyFile = toml::from_str(&contents)?;
       validate_policy(&policy)?;
       Ok(policy)
   }
   ```

### Tests
- Path resolution works on all platforms
- Configuration loading handles errors gracefully
- Invalid policies are rejected

## Phase 6: Integration and Final Validation

### Objectives
- Wire all components together
- Create integration tests
- Validate against design specifications

### Implementation Steps

1. **Main Entry Point** (`src/main.rs`)
   ```rust
   use clap::Parser;
   use cupcake::{cli::{Cli, Commands}, Result};
   
   fn main() -> Result<()> {
       let cli = Cli::parse();
       
       match cli.command {
           Commands::Init { .. } => {
               println!("Init command - implementation pending");
           }
           Commands::Run { .. } => {
               println!("Run command - implementation pending");
           }
           // Handle other commands
       }
       
       Ok(())
   }
   ```

2. **Integration Tests** (`tests/`)
   - Policy file parsing tests
   - CLI invocation tests
   - Error handling scenarios

### Final Validation Checklist

- [ ] All modules compile without warnings
- [ ] All policy schema types match `policy-schema.md` exactly
- [ ] All hook event types match `hook-events.md` exactly
- [ ] CLI interface matches design specifications
- [ ] Core error handling is in place
- [ ] Project structure follows architecture.md
- [ ] All tests pass
- [ ] Documentation comments on public APIs

## Testing Strategy

### Unit Tests
- Each data type has serialization tests
- Error types have display tests
- Path utilities work cross-platform

### Integration Tests
- Policy files parse correctly
- CLI commands accept proper arguments
- Invalid inputs produce appropriate errors

### Quality Standards
- Zero compiler warnings
- All public types have documentation
- Code follows Rust idioms and best practices
- Clear separation of concerns between modules

## Success Metrics

1. **Compilation**: Project builds with `cargo build --release`
2. **Tests**: All tests pass with `cargo test`
3. **Linting**: No issues from `cargo clippy -- -D warnings`
4. **Structure**: Module organization matches design exactly
5. **Types**: All schema types implemented and tested
6. **CLI**: All commands parse arguments correctly

## Next Steps

Upon completion of Plan 001:
- Plan 002 can implement the runtime evaluation engine
- Plan 003 can add user lifecycle features
- Plan 004 can focus on hardening and optimization

This foundation ensures all subsequent work builds on solid, type-safe ground.