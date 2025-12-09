//! Command path extractor
//!
//! Extracts target paths from shell commands for policy evaluation.
//! Uses shell-words for proper shell parsing (handles quotes, escapes, etc.).
//! Glob patterns are stripped to get the parent directory.
//!
//! Available to policies as `input.affected_parent_directories`.

use std::path::PathBuf;
use tracing::trace;

/// Extract target paths from a shell command
///
/// Parses command arguments using proper shell parsing rules and returns path-like values.
/// Handles quoted arguments correctly (e.g., `"My Documents/file.txt"` is parsed as one token).
/// Glob patterns are stripped to their parent directory.
///
/// # Examples
///
/// - `rm -rf /home/user/*` → `["/home/user/"]`
/// - `rm -rf "/tmp/protected"` → `["/tmp/protected"]` (quotes stripped by parser)
/// - `rm -rf "My Documents/file.txt"` → `["My Documents/file.txt"]`
/// - `ls /tmp/*` → `["/tmp/"]`
/// - `cat /etc/passwd` → `["/etc/passwd"]`
/// - `echo "hello"` → `[]`
pub fn extract_target_paths(command: &str) -> Vec<PathBuf> {
    // Use shell-words for proper shell parsing (handles quotes, escapes, etc.)
    let parts = match shell_words::split(command) {
        Ok(parts) => parts,
        Err(_) => {
            // Fall back to simple split on parse error (e.g., unmatched quotes)
            trace!("shell_words parse failed, falling back to simple split");
            command
                .split_whitespace()
                .map(String::from)
                .collect::<Vec<_>>()
        }
    };

    if parts.is_empty() {
        return Vec::new();
    }

    let mut paths = Vec::new();
    for part in parts.iter().skip(1) {
        // Skip flags
        if part.starts_with('-') {
            continue;
        }

        // Skip common non-path arguments
        if part == "--" {
            continue;
        }

        // Skip numeric-only arguments (e.g., permissions like 755)
        if part.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        // Only extract tokens that look like paths
        // Must contain path separator, start with path-like prefix, or be a glob pattern
        let looks_like_path = part.contains('/')
            || part.contains('\\')
            || part.starts_with('.')
            || part.starts_with('$')
            || part.starts_with('~')
            || part.contains('*')
            || part.contains('?')
            || part.contains('[');

        if !looks_like_path {
            continue;
        }

        // Handle glob patterns - extract parent directory
        let path_str = strip_glob_to_parent(part);

        if !path_str.is_empty() {
            trace!("Extracted path: {} from {}", path_str, part);
            paths.push(PathBuf::from(path_str));
        }
    }

    paths
}

/// Strip glob patterns to get the parent directory
///
/// - `/home/user/*` → `/home/user/`
/// - `/home/user/*.txt` → `/home/user/`
/// - `/home/user/**/*.rs` → `/home/user/`
/// - `/home/user/file.txt` → `/home/user/file.txt` (no glob, unchanged)
fn strip_glob_to_parent(path: &str) -> String {
    // Find first glob character
    if let Some(glob_pos) = path.find(['*', '?', '[']) {
        // Find the last path separator before the glob
        let before_glob = &path[..glob_pos];
        if let Some(sep_pos) = before_glob.rfind('/') {
            // Return path up to and including the separator
            return path[..=sep_pos].to_string();
        } else if let Some(sep_pos) = before_glob.rfind('\\') {
            // Windows path separator
            return path[..=sep_pos].to_string();
        } else {
            // Glob at start (e.g., "*.txt") - treat as current directory
            return ".".to_string();
        }
    }

    // No glob - return as-is
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rm_rf_with_glob() {
        let paths = extract_target_paths("rm -rf /home/user/*");
        assert_eq!(paths, vec![PathBuf::from("/home/user/")]);
    }

    #[test]
    fn test_rm_rf_directory() {
        let paths = extract_target_paths("rm -rf /home/user/projects");
        assert_eq!(paths, vec![PathBuf::from("/home/user/projects")]);
    }

    #[test]
    fn test_rm_rf_multiple_paths() {
        let paths = extract_target_paths("rm -rf /tmp/foo /tmp/bar");
        assert_eq!(
            paths,
            vec![PathBuf::from("/tmp/foo"), PathBuf::from("/tmp/bar"),]
        );
    }

    #[test]
    fn test_rm_without_recursive() {
        // rm without -r still extracts paths for safety
        let paths = extract_target_paths("rm /tmp/file.txt");
        assert_eq!(paths, vec![PathBuf::from("/tmp/file.txt")]);
    }

    #[test]
    fn test_chmod_recursive() {
        let paths = extract_target_paths("chmod -R 755 /var/www/");
        assert_eq!(paths, vec![PathBuf::from("/var/www/")]);
    }

    #[test]
    fn test_chmod_with_path() {
        let paths = extract_target_paths("chmod 755 /var/www/file.txt");
        assert_eq!(paths, vec![PathBuf::from("/var/www/file.txt")]);
    }

    #[test]
    fn test_ls_with_glob() {
        let paths = extract_target_paths("ls -la /home/user/*");
        assert_eq!(paths, vec![PathBuf::from("/home/user/")]);
    }

    #[test]
    fn test_cat_command() {
        let paths = extract_target_paths("cat /etc/passwd");
        assert_eq!(paths, vec![PathBuf::from("/etc/passwd")]);
    }

    #[test]
    fn test_echo_no_paths() {
        // echo with a simple string - no paths
        let paths = extract_target_paths("echo \"hello\"");
        assert!(paths.is_empty());
    }

    #[test]
    fn test_glob_double_star() {
        let paths = extract_target_paths("rm -rf /project/**/*.log");
        assert_eq!(paths, vec![PathBuf::from("/project/")]);
    }

    #[test]
    fn test_glob_question_mark() {
        let paths = extract_target_paths("rm -rf /tmp/file?.txt");
        assert_eq!(paths, vec![PathBuf::from("/tmp/")]);
    }

    #[test]
    fn test_glob_bracket() {
        let paths = extract_target_paths("rm -rf /tmp/file[0-9].txt");
        assert_eq!(paths, vec![PathBuf::from("/tmp/")]);
    }

    #[test]
    fn test_glob_at_start() {
        let paths = extract_target_paths("rm -rf *.bak");
        assert_eq!(paths, vec![PathBuf::from(".")]);
    }

    #[test]
    fn test_mv_command() {
        let paths = extract_target_paths("mv /source/dir /dest/dir");
        assert_eq!(
            paths,
            vec![PathBuf::from("/source/dir"), PathBuf::from("/dest/dir"),]
        );
    }

    #[test]
    fn test_variable_path() {
        // Variables are extracted as literal strings
        let paths = extract_target_paths("rm -rf $path");
        assert_eq!(paths, vec![PathBuf::from("$path")]);
    }

    #[test]
    fn test_empty_command() {
        let paths = extract_target_paths("");
        assert!(paths.is_empty());
    }

    // ========== Quoted path tests (shell-words handles these correctly) ==========

    #[test]
    fn test_quoted_path_single_word() {
        // shell-words strips quotes, so we get the actual path
        let paths = extract_target_paths("rm -rf \"/tmp/protected\"");
        assert_eq!(paths, vec![PathBuf::from("/tmp/protected")]);
    }

    #[test]
    fn test_quoted_path_with_spaces() {
        // Multi-word quoted paths are now handled correctly
        let paths = extract_target_paths("rm -rf \"My Documents/file.txt\"");
        assert_eq!(paths, vec![PathBuf::from("My Documents/file.txt")]);
    }

    #[test]
    fn test_single_quoted_path() {
        // Single quotes work the same way
        let paths = extract_target_paths("rm -rf '/tmp/protected'");
        assert_eq!(paths, vec![PathBuf::from("/tmp/protected")]);
    }

    #[test]
    fn test_mixed_quoted_paths() {
        // Mix of quoted and unquoted paths
        let paths = extract_target_paths("mv \"/source/my file\" /dest/dir");
        assert_eq!(
            paths,
            vec![PathBuf::from("/source/my file"), PathBuf::from("/dest/dir"),]
        );
    }

    #[test]
    fn test_echo_hello_world() {
        // echo "hello world" - no paths (hello world doesn't contain /)
        let paths = extract_target_paths("echo \"hello world\"");
        assert!(paths.is_empty());
    }

    // ========== Strip glob tests ==========

    #[test]
    fn test_strip_glob_no_glob() {
        assert_eq!(
            strip_glob_to_parent("/home/user/file.txt"),
            "/home/user/file.txt"
        );
    }

    #[test]
    fn test_strip_glob_star() {
        assert_eq!(strip_glob_to_parent("/home/user/*"), "/home/user/");
    }

    #[test]
    fn test_strip_glob_nested() {
        assert_eq!(
            strip_glob_to_parent("/home/user/src/**/*.rs"),
            "/home/user/src/"
        );
    }
}
