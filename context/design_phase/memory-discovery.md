# Cupcake Memory Discovery Pattern

## Overview

Cupcake's `init` command must follow Claude Code's exact memory discovery pattern to ensure policies are generated from the same CLAUDE.md files that Claude Code uses.

## Discovery Algorithm

### 1. Upward Recursive Discovery

Starting from the current working directory (CWD), Cupcake walks up the directory tree:

```
/home/user/project/src/components/  (CWD)
         ↑ Check for CLAUDE.md / CLAUDE.local.md
/home/user/project/src/
         ↑ Check for CLAUDE.md / CLAUDE.local.md
/home/user/project/
         ↑ Check for CLAUDE.md / CLAUDE.local.md
/home/user/
         ↑ Check for CLAUDE.md / CLAUDE.local.md
/home/
         ↑ Check for CLAUDE.md / CLAUDE.local.md
/        (Stop - root directory)
```

**Implementation**:
```rust
fn discover_upward_claude_files(start_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut current = start_dir;
    
    loop {
        // Check for CLAUDE.md
        let claude_md = current.join("CLAUDE.md");
        if claude_md.exists() {
            files.push(claude_md);
        }
        
        // Check for CLAUDE.local.md (deprecated but still supported)
        let claude_local = current.join("CLAUDE.local.md");
        if claude_local.exists() {
            files.push(claude_local);
        }
        
        // Move up one directory
        match current.parent() {
            Some(parent) if parent != Path::new("/") => current = parent,
            _ => break,
        }
    }
    
    files.reverse(); // Return in top-down order
    files
}
```

### 2. Subtree Discovery

CLAUDE.md files in subdirectories are discovered but loaded lazily (only when Claude reads files in those directories). For `cupcake init`, we need to discover ALL of them upfront:

```
/home/user/project/  (CWD)
    ├── CLAUDE.md  ✓ (found in upward search)
    ├── src/
    │   ├── CLAUDE.md  ✓ (subtree discovery)
    │   └── components/
    │       └── CLAUDE.md  ✓ (subtree discovery)
    └── docs/
        └── CLAUDE.md  ✓ (subtree discovery)
```

**Implementation**:
```rust
fn discover_subtree_claude_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_dir() && !is_hidden(&path) {
                    // Check for CLAUDE.md in this directory
                    let claude_md = path.join("CLAUDE.md");
                    if claude_md.exists() {
                        files.push(claude_md);
                    }
                    
                    // Recurse into subdirectory
                    walk_dir(&path, files);
                }
            }
        }
    }
    
    walk_dir(root, &mut files);
    files
}
```

### 3. Import Resolution

CLAUDE.md files can import other files using `@path/to/file` syntax:

```markdown
# In CLAUDE.md
See @README for project overview and @package.json for commands.

# Additional Instructions
- git workflow @docs/git-instructions.md
- personal preferences @~/.claude/my-preferences.md
```

**Import Rules**:
- Both relative and absolute paths supported
- Imports not evaluated in code blocks/spans
- Max recursion depth: 5 hops
- Circular imports must be detected and prevented

**Implementation**:
```rust
fn resolve_imports(
    content: &str, 
    base_path: &Path, 
    visited: &mut HashSet<PathBuf>,
    depth: usize
) -> Result<String> {
    if depth > 5 {
        return Err("Max import depth exceeded");
    }
    
    let import_regex = Regex::new(r"@([^\s\n]+)")?;
    let mut expanded = content.to_string();
    
    for cap in import_regex.captures_iter(content) {
        let import_path = &cap[1];
        let resolved = resolve_path(import_path, base_path)?;
        
        // Prevent circular imports
        if !visited.insert(resolved.clone()) {
            continue;
        }
        
        if resolved.exists() {
            let imported_content = fs::read_to_string(&resolved)?;
            let processed = resolve_imports(
                &imported_content, 
                resolved.parent().unwrap(),
                visited,
                depth + 1
            )?;
            
            expanded = expanded.replace(&cap[0], &processed);
        }
    }
    
    Ok(expanded)
}
```

## Complete Discovery Flow

The complete discovery process for `cupcake init`:

```rust
pub fn discover_all_claude_content() -> Result<String> {
    let cwd = env::current_dir()?;
    let mut all_content = String::new();
    let mut visited = HashSet::new();
    
    // 1. Discover upward from CWD
    let upward_files = discover_upward_claude_files(&cwd);
    
    // 2. Discover in subtrees
    let subtree_files = discover_subtree_claude_files(&cwd);
    
    // 3. Combine all discovered files
    let mut all_files = upward_files;
    all_files.extend(subtree_files);
    
    // 4. Process each file and resolve imports
    for file in all_files {
        all_content.push_str(&format!("\n\n===== {} =====\n", file.display()));
        
        let content = fs::read_to_string(&file)?;
        let expanded = resolve_imports(
            &content,
            file.parent().unwrap(),
            &mut visited,
            0
        )?;
        
        all_content.push_str(&expanded);
    }
    
    Ok(all_content)
}
```

## Priority and Ordering

Files are processed in this order for policy generation:

1. **Upward files** (root → CWD)
   - `/home/CLAUDE.md`
   - `/home/user/CLAUDE.md`
   - `/home/user/project/CLAUDE.md`

2. **Subtree files** (alphabetical by path)
   - `/home/user/project/docs/CLAUDE.md`
   - `/home/user/project/src/CLAUDE.md`
   - `/home/user/project/src/components/CLAUDE.md`

This ensures:
- Higher-level rules can be overridden by more specific ones
- Consistent, predictable ordering
- Matches Claude Code's own loading behavior

## Edge Cases

### Hidden Directories
Skip directories starting with `.` (except `.claude/`):
```rust
fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.') && n != ".claude")
        .unwrap_or(false)
}
```

### Symbolic Links
Follow symlinks but track visited paths to prevent loops:
```rust
let canonical = path.canonicalize()?;
if !visited.insert(canonical.clone()) {
    return; // Already visited
}
```

### Large Repositories
For performance in large repos:
- Implement parallel directory walking
- Add configurable depth limits
- Skip known large directories (node_modules, .git, etc.)

## Testing Discovery

Test cases should verify:

1. **Upward discovery** stops at root
2. **Subtree discovery** finds nested files
3. **Import resolution** handles all path types
4. **Circular imports** are prevented
5. **Max depth** is enforced
6. **Hidden directories** are skipped
7. **Missing imports** are handled gracefully