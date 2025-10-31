//! Symlink resolution module for detecting and resolving symbolic links
//!
//! This module implements TOB-4 defense by detecting when file paths are
//! symbolic links and resolving them to their canonical target paths.

use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

/// Symlink resolver for detecting and resolving symbolic links
pub struct SymlinkResolver;

impl SymlinkResolver {
    /// Check if a path is a symbolic link
    ///
    /// Uses `symlink_metadata()` which doesn't follow the link, allowing
    /// detection of symlinks even when the target doesn't exist (dangling symlinks).
    ///
    /// Returns `true` if the path is a symlink, `false` otherwise.
    pub fn is_symlink(path: &Path) -> bool {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                let is_link = metadata.is_symlink();
                if is_link {
                    debug!("Detected symlink: {:?}", path);
                }
                is_link
            }
            Err(e) => {
                // File doesn't exist or can't access metadata
                debug!("Could not check if path is symlink {:?}: {}", path, e);
                false
            }
        }
    }

    /// Resolve a symbolic link to its canonical target path
    ///
    /// Handles:
    /// - Relative paths (resolved against cwd if provided)
    /// - Absolute paths
    /// - Nested symlinks (resolves all levels)
    /// - Dangling symlinks (gracefully returns None)
    /// - Permission errors (gracefully returns None)
    ///
    /// Returns the canonical path if resolution succeeds, None otherwise.
    pub fn resolve_path(path: &Path, cwd: Option<&Path>) -> Option<PathBuf> {
        // Resolve the path relative to the working directory if provided
        let resolved_path = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(cwd) = cwd {
            cwd.join(path)
        } else {
            path.to_path_buf()
        };

        debug!("Attempting to resolve symlink: {:?}", resolved_path);

        // Try to canonicalize the path (follows all symlinks to final target)
        match fs::canonicalize(&resolved_path) {
            Ok(canonical) => {
                debug!(
                    "Successfully resolved symlink: {:?} -> {:?}",
                    resolved_path, canonical
                );
                Some(canonical)
            }
            Err(e) => {
                // This can fail for several reasons:
                // - Dangling symlink (target doesn't exist)
                // - Permission denied
                // - Path doesn't exist
                debug!("Could not resolve symlink {:?}: {}", resolved_path, e);

                // Try read_link as fallback for dangling symlinks
                if let Ok(target) = fs::read_link(&resolved_path) {
                    debug!(
                        "Read dangling symlink target: {:?} -> {:?}",
                        resolved_path, target
                    );
                    // Return the target path even if it doesn't exist
                    // This allows policies to check the *intended* target
                    return Some(if target.is_absolute() {
                        target
                    } else if let Some(parent) = resolved_path.parent() {
                        parent.join(target)
                    } else {
                        target
                    });
                }

                // Try parent directory canonicalization as last resort
                // This handles Write operations to non-existent files
                if let Some(parent) = resolved_path.parent() {
                    if parent.exists() {
                        if let Ok(canonical_parent) = fs::canonicalize(parent) {
                            if let Some(filename) = resolved_path.file_name() {
                                debug!(
                                    "Using parent directory canonicalization: {:?} + {:?}",
                                    canonical_parent, filename
                                );
                                return Some(canonical_parent.join(filename));
                            }
                        }
                    }
                }

                None
            }
        }
    }

    /// Attach canonical path metadata to the event JSON (TOB-4 always-on)
    ///
    /// Adds:
    /// - `is_symlink`: Boolean flag indicating if the path was a symlink
    /// - `resolved_file_path`: The canonical path (always present for all files)
    /// - `original_file_path`: The original path (for reference)
    ///
    /// This allows policies to always use canonical paths, preventing bypass attempts.
    pub fn attach_metadata(
        event: &mut serde_json::Value,
        original_path: &str,
        resolved_path: &Path,
        is_symlink: bool,
    ) {
        if let Some(obj) = event.as_object_mut() {
            // Add the is_symlink flag (true if symlink, false if regular file)
            obj.insert(
                "is_symlink".to_string(),
                serde_json::Value::Bool(is_symlink),
            );

            // Add the resolved canonical path (ALWAYS present)
            obj.insert(
                "resolved_file_path".to_string(),
                serde_json::Value::String(resolved_path.to_string_lossy().to_string()),
            );

            // Add the original path for reference
            obj.insert(
                "original_file_path".to_string(),
                serde_json::Value::String(original_path.to_string()),
            );

            debug!(
                "Attached canonical path metadata to event: {} -> {:?} (symlink: {})",
                original_path, resolved_path, is_symlink
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    #[test]
    fn test_is_symlink_detects_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let link_path = temp_dir.path().join("test_link");
        let target_path = temp_dir.path().join("target");

        // Create target file
        fs::write(&target_path, "test").unwrap();

        // Create symlink
        symlink(&target_path, &link_path).unwrap();

        assert!(SymlinkResolver::is_symlink(&link_path));
        assert!(!SymlinkResolver::is_symlink(&target_path));
    }

    #[test]
    fn test_is_symlink_detects_dangling_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let link_path = temp_dir.path().join("dangling_link");
        let nonexistent = "/tmp/nonexistent_target_12345";

        // Create dangling symlink (target doesn't exist)
        symlink(nonexistent, &link_path).unwrap();

        // Should still detect it as a symlink
        assert!(SymlinkResolver::is_symlink(&link_path));
    }

    #[test]
    fn test_is_symlink_returns_false_for_nonexistent() {
        let nonexistent_path = Path::new("/tmp/this_path_does_not_exist_12345");
        assert!(!SymlinkResolver::is_symlink(nonexistent_path));
    }

    #[test]
    fn test_resolve_path_follows_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let link_path = temp_dir.path().join("link");
        let target_path = temp_dir.path().join("target");

        // Create target file
        fs::write(&target_path, "test").unwrap();

        // Create symlink
        symlink(&target_path, &link_path).unwrap();

        let resolved = SymlinkResolver::resolve_path(&link_path, None).unwrap();
        assert_eq!(resolved, fs::canonicalize(&target_path).unwrap());
    }

    #[test]
    fn test_resolve_path_handles_relative_paths() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("target");
        let link_name = "relative_link";
        let link_path = temp_dir.path().join(link_name);

        // Create target
        fs::write(&target_path, "test").unwrap();

        // Create symlink
        symlink(&target_path, &link_path).unwrap();

        // Resolve relative to cwd
        let resolved =
            SymlinkResolver::resolve_path(Path::new(link_name), Some(temp_dir.path())).unwrap();
        assert_eq!(resolved, fs::canonicalize(&target_path).unwrap());
    }

    #[test]
    fn test_resolve_path_handles_dangling_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let link_path = temp_dir.path().join("dangling");
        let nonexistent = temp_dir.path().join("nonexistent_target");

        // Create dangling symlink
        symlink(&nonexistent, &link_path).unwrap();

        // Should still return the target path (even though it doesn't exist)
        let resolved = SymlinkResolver::resolve_path(&link_path, None);
        assert!(resolved.is_some());
        let resolved_path = resolved.unwrap();
        assert!(resolved_path
            .to_string_lossy()
            .contains("nonexistent_target"));
    }

    #[test]
    fn test_resolve_path_returns_none_for_nonexistent() {
        let nonexistent_path = Path::new("/tmp/this_path_does_not_exist_12345");
        // Always-on approach: If parent dir exists (/tmp/), will return canonical parent + filename
        // This is CORRECT behavior for Write operations to new files
        let result = SymlinkResolver::resolve_path(nonexistent_path, None);
        if let Some(resolved) = result {
            // Should be /tmp/this_path_does_not_exist_12345 (canonical parent + filename)
            assert!(resolved
                .to_string_lossy()
                .contains("this_path_does_not_exist_12345"));
        } else {
            // Only returns None if parent doesn't exist either
            panic!("Expected Some(path) due to parent directory fallback");
        }
    }

    #[test]
    fn test_attach_metadata() {
        use serde_json::json;

        let mut event = json!({
            "tool_name": "Write",
            "tool_input": {
                "file_path": "link.txt"
            }
        });

        let resolved = PathBuf::from("/tmp/target.txt");
        SymlinkResolver::attach_metadata(&mut event, "link.txt", &resolved, true);

        assert_eq!(event["is_symlink"], json!(true));
        assert_eq!(event["resolved_file_path"], json!("/tmp/target.txt"));
        assert_eq!(event["original_file_path"], json!("link.txt"));
    }
}
