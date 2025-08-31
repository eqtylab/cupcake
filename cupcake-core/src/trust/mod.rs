//! Cupcake Trust System - Cryptographic integrity verification for scripts
//! 
//! This module provides optional script integrity verification to prevent
//! unauthorized modifications between approval and execution.
//! 
//! Design Principles:
//! - Optional by default - zero friction for users who don't need it
//! - Explicit trust updates - no magic, users control when scripts are approved  
//! - Clean integration - follows existing patterns, minimal engine changes
//! - Industry standard crypto - SHA-256 hashing, HMAC-SHA256 signing

pub mod error;
pub mod hasher;
pub mod manifest;
pub mod verifier;

pub use error::TrustError;
pub use manifest::TrustManifest;
pub use verifier::TrustVerifier;


/// Trust system version for future compatibility
pub const TRUST_VERSION: u32 = 1;