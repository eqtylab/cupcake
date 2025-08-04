//! Shell execution feature test module
//!
//! This module contains comprehensive tests for shell command execution functionality,
//! including core execution, security controls, and command specification handling.
//!
//! ## Organization
//! - `core` - Basic shell execution functionality and template substitution
//! - `security` - Security controls, governance, and privilege management
//! - `command_spec` - Command specification configuration and YAML serialization

pub mod command_spec;
pub mod core;
pub mod security;
