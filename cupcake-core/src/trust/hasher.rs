//! Cryptographic hashing utilities for the trust system
//! 
//! Uses SHA-256 for file content hashing and HMAC-SHA256 for manifest integrity

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use hmac::{Hmac, Mac};
use std::io::Read;
use std::path::Path;

type HmacSha256 = Hmac<Sha256>;

/// Hash a file's contents using SHA-256
pub async fn hash_file(path: &Path) -> Result<String> {
    let contents = tokio::fs::read(path)
        .await
        .with_context(|| format!("Failed to read file for hashing: {}", path.display()))?;
    
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    let hash = hasher.finalize();
    
    Ok(format!("sha256:{}", hex::encode(hash)))
}

/// Hash a file's contents synchronously (for non-async contexts)
pub fn hash_file_sync(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open file for hashing: {}", path.display()))?;
    
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192]; // 8KB buffer for streaming
    
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    let hash = hasher.finalize();
    Ok(format!("sha256:{}", hex::encode(hash)))
}

/// Hash a string using SHA-256
pub fn hash_string(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();
    format!("sha256:{}", hex::encode(hash))
}

/// Derive a unique key for HMAC signing based on machine/user/project context
/// 
/// This provides a deterministic key that's unique per installation without
/// requiring external key management infrastructure.
pub fn derive_trust_key(project_path: &Path) -> Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    
    // Mix in version identifier
    hasher.update(b"CUPCAKE_TRUST_V1");
    
    // In test mode with the specific feature, use only deterministic inputs for reproducible tests
    #[cfg(feature = "deterministic-tests")]
    {
        // Use fixed project identifier for deterministic testing
        hasher.update(b"TEST_MODE_FIXED_PROJECT");
        
        // Add a test-specific constant for uniqueness
        hasher.update(b"TEST_MODE_ENTROPY");
    }
    
    // In production or standard builds, use system-specific entropy for security
    #[cfg(not(feature = "deterministic-tests"))]
    {
        // Always use a consistent path representation by converting to string and normalizing
        let path_str = project_path.to_string_lossy();
        // On macOS, normalize symlinked paths consistently 
        let normalized_path_str = if path_str.starts_with("/var/") && !path_str.starts_with("/var/folders/") {
            path_str.to_string()
        } else if path_str.starts_with("/var/folders/") {
            // macOS temp paths - ensure consistent representation
            path_str.replace("/private/var/folders/", "/var/folders/")
        } else {
            path_str.to_string()
        };
        
        // Add executable path
        if let Ok(exe_path) = std::env::current_exe() {
            hasher.update(exe_path.to_string_lossy().as_bytes());
        }
        
        // Add machine ID (best effort - may not be available on all platforms)
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("ioreg")
                .args(&["-rd1", "-c", "IOPlatformExpertDevice"])
                .output()
            {
                hasher.update(&output.stdout);
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            if let Ok(machine_id) = std::fs::read_to_string("/etc/machine-id") {
                hasher.update(machine_id.trim().as_bytes());
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = std::process::Command::new("wmic")
                .args(&["csproduct", "get", "UUID"])
                .output()
            {
                hasher.update(&output.stdout);
            }
        }
        
        // Add username
        if let Ok(username) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
            hasher.update(username.as_bytes());
        }
        
        // Add normalized project path for project-specific keys
        hasher.update(normalized_path_str.as_bytes());
    }
    
    let key_material = hasher.finalize();
    
    // Use the hash as our key (already 32 bytes)
    let mut key = [0u8; 32];
    key.copy_from_slice(&key_material);
    
    Ok(key)
}

/// Compute HMAC-SHA256 for a message using the derived trust key
pub fn compute_hmac(message: &[u8], project_path: &Path) -> Result<String> {
    let key = derive_trust_key(project_path)?;
    
    let mut mac = HmacSha256::new_from_slice(&key)
        .context("Failed to create HMAC instance")?;
    
    mac.update(message);
    let result = mac.finalize();
    
    Ok(format!("hmac-sha256:{}", hex::encode(result.into_bytes())))
}

/// Verify HMAC-SHA256 signature
pub fn verify_hmac(message: &[u8], signature: &str, project_path: &Path) -> Result<bool> {
    // Extract the hex signature from the format string
    let hex_signature = signature
        .strip_prefix("hmac-sha256:")
        .unwrap_or(signature);
    
    let signature_bytes = hex::decode(hex_signature)
        .context("Failed to decode HMAC signature")?;
    
    let key = derive_trust_key(project_path)?;
    
    let mut mac = HmacSha256::new_from_slice(&key)
        .context("Failed to create HMAC instance")?;
    
    mac.update(message);
    
    // Constant-time comparison
    match mac.verify_slice(&signature_bytes) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_hash_string() {
        let hash = hash_string("hello world");
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 7 + 64); // "sha256:" + 64 hex chars
    }
    
    #[test]
    fn test_hash_string_deterministic() {
        let hash1 = hash_string("test content");
        let hash2 = hash_string("test content");
        assert_eq!(hash1, hash2);
    }
    
    #[test]
    fn test_hash_file_sync() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "test file content")?;
        
        let hash = hash_file_sync(temp_file.path())?;
        assert!(hash.starts_with("sha256:"));
        
        Ok(())
    }
    
    #[test]
    fn test_hmac_roundtrip() -> Result<()> {
        let project_path = Path::new("/tmp/test-project");
        let message = b"test message";
        
        let signature = compute_hmac(message, project_path)?;
        assert!(signature.starts_with("hmac-sha256:"));
        
        let valid = verify_hmac(message, &signature, project_path)?;
        assert!(valid);
        
        let invalid = verify_hmac(b"different message", &signature, project_path)?;
        assert!(!invalid);
        
        Ok(())
    }
    
    #[test]
    fn test_derive_key_deterministic() -> Result<()> {
        let project_path = Path::new("/tmp/test-project");
        
        let key1 = derive_trust_key(project_path)?;
        let key2 = derive_trust_key(project_path)?;
        
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);
        
        Ok(())
    }
    
}