# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cupcake is a deterministic policy enforcement engine for Claude Code. This is currently a pre-implementation project with comprehensive design documentation. The codebase follows a well-structured design phase approach with clear implementation plans.

## Project Management

Use the `lnr work` project management system. You will manage plans, and leave tips for other instances of Claude Code.

- Read @context/CLAUDE.md for additional context on how we manage plans for considerable feature development.
- all planning and work tracking are done it the `@context/lnrwrk/` directory

## Development Commands

### Build and Test (once implemented)

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Build release version with optimizations
cargo build --release

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy linter
cargo clippy -- -D warnings
```

### Work Management

The project uses an lnr (linear) work management system. Check `context/lnrwrk/` for current plans:

- Plan 001: Core Domain and Type-Safe Foundation (current)
- Plan 002: Runtime Evaluation and Action System
- Plan 003: User Lifecycle and Integration
- Plan 004: Hardening and Release Readiness

## Architecture

### Core Structure (to be implemented)

- `src/cli/` - Command-line interface using clap
- `src/engine/` - Policy evaluation engine
- `src/config/` - Configuration and policy parsing
- `src/state/` - Session state management
- `src/io/` - File I/O and caching

### Key Design Patterns

1. **Two-Pass Aggregation**: Collect feedback first, then check for hard actions
2. **Binary Caching**: Use bincode for sub-100ms policy loading
3. **Append-Only State**: Session tracking in `.cupcake/state/`
4. **Stateful Awareness**: Remember file reads and test runs across session
5. **Extensible Validation**: Execute project scripts for custom checks

### Performance Requirements

- Sub-100ms response time for all hook operations
- Binary serialization for policy cache
- Minimal file I/O during runtime

## Key Dependencies

- `serde` & `serde_json` - Serialization
- `serde_yaml_ng` - YAML policy file format
- `clap` - CLI framework
- `tokio` - Async runtime
- `anyhow` & `thiserror` - Error handling
- `bincode` - Binary serialization

## Integration with Claude Code

- Hooks configured in `.claude/settings.local.json`
- Policy files in YAML format via `guardrails/cupcake.yaml`
- State tracked in `.cupcake/` directory

## Important Design Documents

- Architecture: `context/design_phase/architecture.md` - Master blueprint
- Policy Schema: `context/design_phase/policy-schema.md` - YAML guardrails specification
- Feedback Model: `context/design_phase/feedback-aggregation.md` - Two-pass evaluation
- Hook Events: `context/design_phase/hook-events.md` - Claude Code lifecycle mapping
- Meta Prompt: `context/design_phase/meta-prompt.md` - AI translation logic for init
- Implementation roadmap in `context/lnrwrk/plans/`

## CLI Commands (once implemented)

### Core Commands

- `cupcake init` - Interactive policy generation from CLAUDE.md files
- `cupcake sync` - Updates Claude Code hooks in .claude/settings.local.json
- `cupcake run` - Runtime enforcement (called automatically by hooks)
- `cupcake validate` - Validates YAML guardrails syntax
- `cupcake audit` - Views audit logs from .cupcake/state/

## Policy File Format

Policies are defined in YAML format in the `guardrails/` directory:

- Root config: `guardrails/cupcake.yaml` with settings and imports
- Policy fragments: `guardrails/policies/*.yaml` organized by hook event
- Conditions: `pattern`, `check`, `state_exists` 
- Actions: `provide_feedback` (soft), `block_with_feedback` (hard), `run_command`
- Two-pass evaluation: Soft feedback aggregated, hard blocks shown immediately

## Development Notes

**MANDATORY**: Before implementing any new core area of Cupcake (engine, CLI commands, state management, etc.), you MUST:

1. Read the relevant design phase documentation in `context/design_phase/`
2. Ensure complete alignment with the documented design
3. If implementation needs to deviate from the design, STOP and discuss with the user first
4. The design documents represent months of careful planning - respect them

- This is a greenfield Rust implementation
- Follow the structured plans in `context/lnrwrk/`
- Prioritize type safety and performance
- Use the documented library versions in `context/lib_docs/`
- Core principle: Sub-100ms response time for all operations
- State management: Append-only tracking in .cupcake/state/
- Binary cache: Use bincode for fast policy loading

Always read entire files to avoid confusion and misinterpretation.
