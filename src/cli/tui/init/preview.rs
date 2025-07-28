use std::fs;
use std::path::Path;
use crate::Result;

/// Load preview content for a file
pub async fn load_preview(path: &Path) -> Result<String> {
    let path = path.to_path_buf();
    
    // Run in blocking task since file I/O is blocking
    tokio::task::spawn_blocking(move || {
        load_preview_sync(&path)
    }).await.map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
}

/// Synchronous preview loading
fn load_preview_sync(path: &Path) -> Result<String> {
    // Check if path exists
    if !path.exists() {
        return Ok(format!("File not found: {}", path.display()));
    }
    
    // If it's a directory, show directory info
    if path.is_dir() {
        return load_directory_preview(path);
    }
    
    // Read file contents
    match fs::read_to_string(path) {
        Ok(content) => {
            // Limit preview to first 50 lines
            let lines: Vec<&str> = content.lines().take(50).collect();
            let preview = lines.join("\n");
            
            if content.lines().count() > 50 {
                Ok(format!("{}\n\n... (truncated, {} more lines)", 
                    preview, 
                    content.lines().count() - 50))
            } else {
                Ok(preview)
            }
        }
        Err(e) => {
            // File might be binary or unreadable
            Ok(format!("Cannot preview file: {}", e))
        }
    }
}

/// Load preview for a directory
fn load_directory_preview(path: &Path) -> Result<String> {
    let mut preview = format!("📁 Directory: {}\n\n", path.display());
    
    let mut entries = Vec::new();
    let mut file_count = 0;
    let mut dir_count = 0;
    
    // Read directory entries
    if let Ok(read_dir) = fs::read_dir(path) {
        for entry in read_dir.flatten() {
            if let Ok(file_type) = entry.file_type() {
                let name = entry.file_name().to_string_lossy().to_string();
                
                if file_type.is_dir() {
                    entries.push(format!("  📁 {}/", name));
                    dir_count += 1;
                } else {
                    entries.push(format!("  📄 {}", name));
                    file_count += 1;
                }
                
                // Limit to 20 entries
                if entries.len() >= 20 {
                    entries.push("  ... (more files)".to_string());
                    break;
                }
            }
        }
    }
    
    preview.push_str(&format!("Contents ({} directories, {} files):\n\n", dir_count, file_count));
    preview.push_str(&entries.join("\n"));
    
    Ok(preview)
}

/// Create a mock preview for testing
pub fn mock_preview(path: &Path) -> String {
    match path.to_str() {
        Some("CLAUDE.md") => {
            r#"# Claude Development Rules

## Testing Standards
- Always write tests first
- Minimum 80% coverage
- Use descriptive test names

## Code Style
- Use 2-space indentation
- Prefer const over let
- Max line length: 100

## Security
- Never commit secrets
- Validate all inputs
- Use parameterized queries"#.to_string()
        }
        Some(".cursor/rules") => {
            r#"# Cursor AI Rules

1. Focus on code clarity
2. Write self-documenting code
3. Add comments for complex logic
4. Keep functions small
5. Use meaningful variable names"#.to_string()
        }
        Some(".aider.conf.yml") => {
            r#"# Aider configuration

model: gpt-4
auto-commits: true
lint-cmd: "cargo clippy"
test-cmd: "cargo test"

# File patterns to watch
watch:
  - "src/**/*.rs"
  - "Cargo.toml""#.to_string()
        }
        _ if path.to_str().unwrap_or("").contains("windsurf") => {
            r#"# Windsurf Rule

This file contains specific rules for the Windsurf agent.

## Guidelines
- Follow project conventions
- Maintain consistency
- Document edge cases"#.to_string()
        }
        _ => format!("Preview for: {}\n\n(File content would appear here)", path.display())
    }
}