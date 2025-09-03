//! Trust manifest structure and script reference parsing
//! 
//! The manifest is the source of truth for what scripts are trusted.
//! It contains hashes of all approved scripts and an HMAC signature for integrity.

use crate::trust::error::TrustError;
use crate::trust::hasher::{hash_file, hash_file_sync, hash_string, compute_hmac, verify_hmac};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Trust verification mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TrustMode {
    /// Trust verification is enabled (default)
    Enabled,
    /// Trust verification is temporarily disabled
    Disabled,
}

impl Default for TrustMode {
    fn default() -> Self {
        TrustMode::Enabled
    }
}

/// Reference to a script that may be executed
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptReference {
    /// Pure inline command (e.g., "npm test", "echo hello")
    Inline(String),
    
    /// Direct file execution (e.g., "./check.sh", "/usr/bin/validate")
    File(PathBuf),
    
    /// Interpreter with script (e.g., "python script.py", "node build.js")
    Complex {
        interpreter: String,
        script_path: PathBuf,
        args: Vec<String>,
    },
}

impl ScriptReference {
    /// Parse a command string into a ScriptReference
    pub fn parse(command: &str, working_dir: &Path) -> Self {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return ScriptReference::Inline(String::new());
        }
        
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        
        // Check for direct file execution (absolute or relative path)
        if trimmed.starts_with('/') || trimmed.starts_with("./") || trimmed.starts_with("../") {
            // It's a direct script path, possibly with arguments
            // Take the first part as the script path
            if let Some(script_path) = parts.first() {
                let path = Path::new(script_path);
                let resolved = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    working_dir.join(path)
                };
                return ScriptReference::File(resolved);
            }
        }
        
        // Check for interpreter + script pattern
        if parts.len() >= 2 {
            let interpreter = parts[0];
            let potential_script = parts[1];
            
            // Common interpreters
            match interpreter {
                "python" | "python3" | "node" | "nodejs" | "ruby" | "perl" | "bash" | "sh" | "zsh" | "php" | "lua" | "julia" => {
                    // Check if second part looks like a script file (not a flag)
                    if !potential_script.starts_with('-') {
                        let script_path = Path::new(potential_script);
                        let resolved = if script_path.is_absolute() {
                            script_path.to_path_buf()
                        } else {
                            working_dir.join(script_path)
                        };
                        
                        // Only classify as Complex if the script file exists or looks like a path
                        if resolved.exists() || potential_script.contains('/') || potential_script.contains('.') {
                            return ScriptReference::Complex {
                                interpreter: interpreter.to_string(),
                                script_path: resolved,
                                args: parts[2..].iter().map(|s| s.to_string()).collect(),
                            };
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Default: treat as inline command
        ScriptReference::Inline(trimmed.to_string())
    }
    
    /// Compute the hash for this script reference
    pub async fn compute_hash(&self) -> Result<String> {
        match self {
            ScriptReference::Inline(cmd) => {
                // Hash the command string itself
                Ok(hash_string(cmd))
            }
            ScriptReference::File(path) => {
                // Hash the file contents
                hash_file(path).await
            }
            ScriptReference::Complex { script_path, .. } => {
                // Hash the script file contents (ignore interpreter and args)
                hash_file(script_path).await
            }
        }
    }
    
    /// Compute hash synchronously (for CLI operations)
    pub fn compute_hash_sync(&self) -> Result<String> {
        match self {
            ScriptReference::Inline(cmd) => {
                Ok(hash_string(cmd))
            }
            ScriptReference::File(path) => {
                hash_file_sync(path)
            }
            ScriptReference::Complex { script_path, .. } => {
                hash_file_sync(script_path)
            }
        }
    }
    
    /// Get the path if this is a file-based script
    pub fn as_path(&self) -> Option<&Path> {
        match self {
            ScriptReference::Inline(_) => None,
            ScriptReference::File(path) => Some(path),
            ScriptReference::Complex { script_path, .. } => Some(script_path),
        }
    }
}

/// Entry for a single script in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEntry {
    /// Type of script reference
    #[serde(rename = "type")]
    pub script_type: String,
    
    /// The original command string
    pub command: String,
    
    /// SHA-256 hash of the script content
    pub hash: String,
    
    /// For file scripts: the resolved absolute path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub absolute_path: Option<PathBuf>,
    
    /// For file scripts: size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    
    /// For file scripts: last modified time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<DateTime<Utc>>,
    
    /// For complex scripts: interpreter details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpreter: Option<String>,
    
    /// For complex scripts: arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
}

impl ScriptEntry {
    /// Create a script entry from a command string
    pub async fn from_command(command: &str, working_dir: &Path) -> Result<Self> {
        let script_ref = ScriptReference::parse(command, working_dir);
        Self::from_reference(&script_ref, command).await
    }
    
    /// Create a script entry from a ScriptReference
    pub async fn from_reference(script_ref: &ScriptReference, original_command: &str) -> Result<Self> {
        let hash = script_ref.compute_hash().await?;
        
        let entry = match script_ref {
            ScriptReference::Inline(_cmd) => ScriptEntry {
                script_type: "inline".to_string(),
                command: original_command.to_string(),
                hash,
                absolute_path: None,
                size: None,
                modified: None,
                interpreter: None,
                args: None,
            },
            ScriptReference::File(path) => {
                let metadata = tokio::fs::metadata(path).await?;
                ScriptEntry {
                    script_type: "file".to_string(),
                    command: original_command.to_string(),
                    hash,
                    absolute_path: Some(path.canonicalize()?),
                    size: Some(metadata.len()),
                    modified: Some(Utc::now()), // Note: Could use actual mtime if needed
                    interpreter: None,
                    args: None,
                }
            }
            ScriptReference::Complex { interpreter, script_path, args } => {
                let metadata = tokio::fs::metadata(script_path).await?;
                ScriptEntry {
                    script_type: "complex".to_string(),
                    command: original_command.to_string(),
                    hash,
                    absolute_path: Some(script_path.canonicalize()?),
                    size: Some(metadata.len()),
                    modified: Some(Utc::now()),
                    interpreter: Some(interpreter.clone()),
                    args: if args.is_empty() { None } else { Some(args.clone()) },
                }
            }
        };
        
        Ok(entry)
    }
    
    /// Get the hash of this script entry
    pub fn hash(&self) -> &str {
        &self.hash
    }
}

/// The complete trust manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustManifest {
    /// Manifest format version
    pub version: u32,
    
    /// When this manifest was created/updated
    pub timestamp: DateTime<Utc>,
    
    /// Trust mode (enabled/disabled)
    #[serde(default)]
    pub mode: TrustMode,
    
    /// Hash of all policy files for completeness
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_hash: Option<String>,
    
    /// All trusted scripts organized by category
    pub scripts: BTreeMap<String, BTreeMap<String, ScriptEntry>>,
    
    /// HMAC signature of the manifest (excluded from signing)
    #[serde(skip)]
    pub hmac: String,
}

impl TrustManifest {
    /// Create a new empty manifest
    pub fn new() -> Self {
        let mut scripts = BTreeMap::new();
        scripts.insert("signals".to_string(), BTreeMap::new());
        scripts.insert("actions".to_string(), BTreeMap::new());
        
        TrustManifest {
            version: crate::trust::TRUST_VERSION,
            timestamp: Utc::now(),
            mode: TrustMode::Enabled,
            policy_hash: None,
            scripts,
            hmac: String::new(),
        }
    }
    
    /// Load manifest from disk
    pub fn load(manifest_path: &Path) -> Result<Self, TrustError> {
        if !manifest_path.exists() {
            return Err(TrustError::NotInitialized);
        }
        
        let content = std::fs::read_to_string(manifest_path)
            .map_err(|e| TrustError::ManifestReadError {
                path: manifest_path.to_path_buf(),
                source: e,
            })?;
        
        // Parse JSON with HMAC at the end
        let (json_content, hmac) = Self::extract_hmac_from_content(&content)?;
        
        let mut manifest: TrustManifest = serde_json::from_str(&json_content)
            .map_err(|e| TrustError::ManifestParseError { source: e })?;
        
        manifest.hmac = hmac;
        
        // Verify HMAC using the original JSON content (not re-serialized)
        let project_path = manifest_path.parent().and_then(|p| p.parent())
            .ok_or_else(|| TrustError::ManifestReadError {
                path: manifest_path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "Invalid manifest path"),
            })?;
        
        if !verify_hmac(json_content.as_bytes(), &manifest.hmac, project_path)
            .map_err(|_| TrustError::ManifestTampered)? {
            return Err(TrustError::ManifestTampered);
        }
        
        Ok(manifest)
    }
    
    /// Save manifest to disk with HMAC
    pub fn save(&mut self, manifest_path: &Path) -> Result<(), TrustError> {
        let project_path = manifest_path.parent().and_then(|p| p.parent())
            .ok_or_else(|| TrustError::ManifestWriteError {
                path: manifest_path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "Invalid manifest path"),
            })?;
        
        // Update timestamp
        self.timestamp = Utc::now();
        
        // Create a copy for serialization without HMAC
        let mut manifest_for_hmac = self.clone();
        manifest_for_hmac.hmac = String::new();
        
        // Serialize to JSON (without HMAC)
        let json_content = serde_json::to_string_pretty(&manifest_for_hmac)
            .map_err(|e| TrustError::ManifestParseError { source: e })?;
        
        // Compute HMAC
        let hmac = compute_hmac(json_content.as_bytes(), project_path)
            .map_err(|_| TrustError::ManifestWriteError {
                path: manifest_path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::Other, "Failed to compute HMAC"),
            })?;
        
        // Write with HMAC appended
        let final_content = format!("{}\n\n/* HMAC: {} */", json_content, hmac);
        
        // Ensure parent directory exists
        if let Some(parent) = manifest_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        
        std::fs::write(manifest_path, final_content)
            .map_err(|e| TrustError::ManifestWriteError {
                path: manifest_path.to_path_buf(),
                source: e,
            })?;
        
        self.hmac = hmac;
        
        Ok(())
    }
    
    
    /// Extract HMAC from saved content
    fn extract_hmac_from_content(content: &str) -> Result<(String, String), TrustError> {
        // Look for HMAC comment at the end
        if let Some(hmac_start) = content.rfind("/* HMAC: ") {
            if let Some(hmac_end) = content[hmac_start..].find(" */") {
                let hmac = content[hmac_start + 9..hmac_start + hmac_end].to_string();
                let json_content = content[..hmac_start].trim().to_string();
                return Ok((json_content, hmac));
            }
        }
        
        // If no HMAC found, treat entire content as JSON (legacy or corrupted)
        Err(TrustError::ManifestTampered)
    }
    
    /// Add or update a script entry
    pub fn add_script(&mut self, category: &str, name: &str, entry: ScriptEntry) {
        self.scripts
            .entry(category.to_string())
            .or_insert_with(BTreeMap::new)
            .insert(name.to_string(), entry);
    }
    
    /// Get a script entry
    pub fn get_script(&self, category: &str, name: &str) -> Option<&ScriptEntry> {
        self.scripts.get(category)?.get(name)
    }
    
    /// Get the creation timestamp
    pub fn created_at(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.timestamp
    }
    
    /// Get all scripts in the manifest
    pub fn scripts(&self) -> &BTreeMap<String, BTreeMap<String, ScriptEntry>> {
        &self.scripts
    }
    
    /// Find a script entry by command
    pub fn find_script_by_command(&self, command: &str) -> Option<(&str, &str, &ScriptEntry)> {
        for (category, scripts) in &self.scripts {
            for (name, entry) in scripts {
                if entry.command == command {
                    return Some((category, name, entry));
                }
            }
        }
        None
    }
    
    /// Check if trust verification is enabled
    pub fn is_enabled(&self) -> bool {
        self.mode == TrustMode::Enabled
    }
    
    /// Set the trust mode (enabled/disabled)
    pub fn set_mode(&mut self, mode: TrustMode) -> Result<()> {
        self.mode = mode;
        self.timestamp = Utc::now();
        // HMAC will be automatically updated when save() is called
        Ok(())
    }
    
    /// Get the current trust mode
    pub fn mode(&self) -> TrustMode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn warn_if_missing_deterministic_flag() {
        #[cfg(not(feature = "deterministic-tests"))]
        {
            eprintln!("\n⚠️  WARNING: Trust tests require --features deterministic-tests flag");
            eprintln!("   Run: cargo test --features deterministic-tests\n");
        }
    }
    
    #[test]
    fn test_script_reference_parse_inline() {
        let working_dir = Path::new("/tmp");
        
        let ref1 = ScriptReference::parse("npm test", working_dir);
        assert!(matches!(ref1, ScriptReference::Inline(cmd) if cmd == "npm test"));
        
        let ref2 = ScriptReference::parse("echo hello world", working_dir);
        assert!(matches!(ref2, ScriptReference::Inline(cmd) if cmd == "echo hello world"));
    }
    
    #[test]
    fn test_script_reference_parse_file() {
        let working_dir = Path::new("/tmp");
        
        let ref1 = ScriptReference::parse("./script.sh", working_dir);
        assert!(matches!(ref1, ScriptReference::File(path) if path == Path::new("/tmp/script.sh")));
        
        let ref2 = ScriptReference::parse("/usr/bin/test", working_dir);
        assert!(matches!(ref2, ScriptReference::File(path) if path == Path::new("/usr/bin/test")));
    }
    
    #[test]
    fn test_script_reference_parse_complex() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("script.py");
        std::fs::write(&script_path, "print('test')").unwrap();
        
        let ref1 = ScriptReference::parse(
            &format!("python {} --flag", script_path.display()),
            temp_dir.path()
        );
        
        match ref1 {
            ScriptReference::Complex { interpreter, script_path: path, args } => {
                assert_eq!(interpreter, "python");
                assert_eq!(path, script_path);
                assert_eq!(args, vec!["--flag"]);
            }
            _ => panic!("Expected Complex variant"),
        }
    }
    
    #[test]
    fn test_script_reference_hash_inline() {
        let ref1 = ScriptReference::Inline("test command".to_string());
        let hash = ref1.compute_hash_sync().unwrap();
        assert!(hash.starts_with("sha256:"));
    }
    
    #[tokio::test]
    async fn test_manifest_roundtrip() {
        warn_if_missing_deterministic_flag();
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join(".cupcake").join(".trust");
        std::fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();
        
        // Create and save manifest
        let mut manifest = TrustManifest::new();
        let entry = ScriptEntry {
            script_type: "inline".to_string(),
            command: "npm test".to_string(),
            hash: "sha256:test".to_string(),
            absolute_path: None,
            size: None,
            modified: None,
            interpreter: None,
            args: None,
        };
        manifest.add_script("signals", "test_signal", entry);
        manifest.save(&manifest_path).unwrap();
        
        // Load and verify
        let loaded = TrustManifest::load(&manifest_path).unwrap();
        assert_eq!(loaded.version, crate::trust::TRUST_VERSION);
        assert!(loaded.get_script("signals", "test_signal").is_some());
    }
}