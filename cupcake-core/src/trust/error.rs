//! Trust system error types with clear, actionable messages

use std::path::PathBuf;
use thiserror::Error;

/// Trust system specific errors
#[derive(Error, Debug)]
pub enum TrustError {
    /// Trust has not been initialized
    #[error("Trust mode is not initialized.\n\nTo enable script integrity verification, run:\n  cupcake trust init")]
    NotInitialized,
    
    /// Script has been modified since trust was established
    #[error("Script integrity violation detected!\n\nScript: {path}\nExpected hash: {expected}\nActual hash:   {actual}\n\nThis script has been modified since it was trusted.\n\nTo approve this change, run:\n  cupcake trust update\n\nTo restore the original script, revert your changes.")]
    ScriptModified {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    
    /// Trust manifest has been tampered with
    #[error("SECURITY ALERT: Trust manifest has been tampered with!\n\nThe trust manifest's HMAC signature verification failed.\nThis is a critical security event that may indicate an attack.\n\nRecommended actions:\n1. Check for unauthorized access to your system\n2. Review recent changes to .cupcake/.trust\n3. Re-initialize trust with: cupcake trust reset --force && cupcake trust init")]
    ManifestTampered,
    
    /// Script is not in the trust manifest
    #[error("Script not found in trust manifest: {path}\n\nThis script has not been approved for execution.\n\nTo add this script to the trust manifest, run:\n  cupcake trust update")]
    ScriptNotTrusted { path: PathBuf },
    
    /// Failed to read a script file for verification
    #[error("Failed to read script for verification: {path}")]
    ScriptNotFound {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    
    /// Failed to read the trust manifest
    #[error("Failed to read trust manifest from {path}")]
    ManifestReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    
    /// Failed to parse the trust manifest
    #[error("Failed to parse trust manifest (corrupted or invalid format)")]
    ManifestParseError {
        #[source]
        source: serde_json::Error,
    },
    
    /// Failed to write the trust manifest
    #[error("Failed to write trust manifest to {path}")]
    ManifestWriteError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

// Note: We don't implement From<TrustError> for anyhow::Error because
// anyhow already has a blanket implementation for all Error types.
// TrustError implements Error via thiserror, so it automatically works with anyhow.

/// Log security-critical trust errors
impl TrustError {
    pub fn log_if_security_critical(&self) {
        match self {
            TrustError::ManifestTampered | TrustError::ScriptModified { .. } => {
                tracing::error!(target: "security", "TRUST VIOLATION: {}", self);
            }
            _ => {}
        }
    }
}