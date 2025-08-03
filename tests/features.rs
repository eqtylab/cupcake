//! Features integration test suite
//! 
//! This file enables running all feature-specific tests with:
//! cargo test --test features

mod common;

mod features {
    pub mod context_injection;
    pub mod shell_execution;
}