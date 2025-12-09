//! Command path extractor
//!
//! Extracts target paths from shell commands for policy evaluation.
//! Glob patterns are stripped to get the parent directory.
//!
//! Available to policies as `input.affected_parent_directories`.

use std::path::PathBuf;
use tracing::trace;

/// Extract target paths from a shell command
///
/// Parses command arguments and returns path-like values.
/// Glob patterns are stripped to their parent directory.
///
/// # Examples
///
/// - `rm -rf /home/user/*` → `["/home/user/"]`
/// - `ls /tmp/*` → `["/tmp/"]`
/// - `cat /etc/passwd` → `["/etc/passwd"]`
/// - `echo "hello"` → `[]`
pub fn extract_target_paths(command: &str) -> Vec<PathBuf> {
    let parts: Vec<&str> = command.split_whitespace().collect();
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
        if *part == "--" {
            continue;
        }

        // Skip numeric-only arguments (e.g., permissions like 755)
        if part.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        // Skip quoted strings that don't look like paths
        if (part.starts_with('"') && part.ends_with('"'))
            || (part.starts_with('\'') && part.ends_with('\''))
        {
            let inner = &part[1..part.len() - 1];
            if !inner.contains('/') && !inner.contains('\\') {
                continue;
            }
        }

        // Skip partial quoted strings (from naive whitespace splitting)
        // e.g., `echo "hello world"` splits to ["echo", "\"hello", "world\""]
        // Neither "\"hello" nor "world\"" are paths
        if (part.starts_with('"') && !part.ends_with('"'))
            || (part.ends_with('"') && !part.starts_with('"'))
            || (part.starts_with('\'') && !part.ends_with('\''))
            || (part.ends_with('\'') && !part.starts_with('\''))
        {
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
