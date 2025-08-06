//! Features integration test suite
//!
//! This file enables running all feature-specific tests with:
//! cargo test --test features

mod common;

mod features {
    pub mod actions;
    pub mod array_command_execution;
    pub mod config;
    pub mod context_injection;
    pub mod context_injection_modes;
    pub mod hook_events;
    pub mod integration;
    pub mod performance;
    pub mod phase2_alignment;
    pub mod policy_evaluation;
    pub mod policy_matching;
    pub mod response_format;
    pub mod security;
    pub mod shell_execution;
    pub mod tui;
}
