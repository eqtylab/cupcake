# Trust System Discovery Log

This document captures findings from investigating how to implement the trust system in Cupcake. Each discovery task explores critical questions that will inform our implementation approach.

---

## Discovery Tasks

### 1. Current Script Execution Flow
**Question**: How does the engine currently execute scripts? What's the full path from guidebook to execution?

**Investigation Points**:
- [ ] Trace `guidebook.rs` signal/action loading
- [ ] Find script execution in `engine/mod.rs`
- [ ] Understand async execution patterns
- [ ] Document all execution entry points

**Findings**:

1. **Signal Execution** (`guidebook.rs:219-235`):
   - Always executed via `sh -c` command
   - Timeout support (default 5 seconds)
   - Output parsed as JSON if possible, otherwise stored as string
   - Concurrent execution for multiple signals
   - Returns Result<Value> that gets merged into input

2. **Action Execution** (`mod.rs:493-553`):
   - Fire-and-forget via `tokio::spawn`
   - Current heuristic detection: checks if starts with `/` or `./` and lacks shell operators
   - If detected as script: executed directly as binary
   - Otherwise: executed via `sh -c`
   - Working directory logic is complex (triple parent for scripts?)

3. **Command Types in Guidebook**:
   ```yaml
   # Inline shell command
   command: "npm test"
   
   # Direct script reference
   command: "./scripts/check.sh"
   
   # Complex command with interpreter
   command: "python ~/analyzer.py --flag"
   ```

4. **Execution Points**:
   - **Signals**: During `gather_signals()` phase, before policy evaluation
   - **Actions**: After policy evaluation, based on decision type
   - Both use async patterns, but signals are awaited, actions are not

**Implications**:
- Need to intercept at 3 points: `execute_signal`, `execute_single_action`, and convention-based discovery
- Current heuristic detection is fragile and was already identified as an issue
- Trust verification must handle both `sh -c` wrapped commands and direct execution
- Path resolution happens at different stages for signals vs actions

---

### 2. SHA-256 Hashing Performance
**Question**: What's the performance impact of hashing files of various sizes?

**Test Cases**:
- [ ] Small scripts (<1KB)
- [ ] Medium scripts (1-10KB)  
- [ ] Large scripts (10-100KB)
- [ ] Very large scripts (>100KB)
- [ ] Multiple files in parallel

**Benchmark Results**:
```
TODO: Add benchmark results
```

**Performance Targets**:
- Acceptable overhead: <10ms for typical script set
- Consider caching strategy if >10ms

---

### 3. Optional Trust Architecture
**Question**: How can we make trust optional without polluting the codebase with if statements?

**Design Patterns to Explore**:
- [ ] Option<TrustVerifier> pattern
- [ ] Trait-based approach (TrustVerifier trait with Noop/Real impls)
- [ ] Strategy pattern
- [ ] Feature flags

**Evaluation Criteria**:
- Minimal runtime overhead when disabled
- Clean integration points
- Testability
- No code duplication

**Findings**:
- Current codebase uses `Option<T>` pattern extensively (guidebook, wasm_runtime, metadata)
- Pattern: `if let Some(x) = &self.optional_field { ... }`
- No trait abstractions for optional features currently

**Proposed Design**:

```rust
// In Engine struct
pub struct Engine {
    // ... existing fields ...
    
    /// Optional trust verifier - None means trust is disabled
    trust_verifier: Option<TrustVerifier>,
}

// Clean integration pattern
impl Engine {
    async fn execute_signal(&self, signal: &SignalConfig) -> Result<Value> {
        // Trust verification - single line, no pollution
        if let Some(trust) = &self.trust_verifier {
            trust.verify_script(&signal.command)?;
        }
        
        // Normal execution continues
        self.execute_signal_internal(signal).await
    }
}

// Alternative: Extension trait pattern for zero runtime cost when disabled
trait TrustExt {
    fn verify_if_enabled(&self, command: &str) -> Result<()>;
}

impl TrustExt for Option<TrustVerifier> {
    fn verify_if_enabled(&self, command: &str) -> Result<()> {
        match self {
            Some(verifier) => verifier.verify_script(command),
            None => Ok(())  // No-op when disabled
        }
    }
}
```

**Benefits**:
- Consistent with existing patterns
- Zero overhead when disabled (branch prediction friendly)
- Clean integration points (2-3 lines per execution site)
- Type safe - can't accidentally use trust features when disabled

---

### 4. Symlink and Path Edge Cases
**Question**: How do we handle symlinks, relative paths, and path traversal?

**Test Scenarios**:
- [ ] Symlinked scripts
- [ ] Scripts with relative paths (../)
- [ ] Scripts in PATH
- [ ] Scripts with spaces in path
- [ ] Non-existent scripts
- [ ] Scripts that are actually directories

**Current Behavior**:
```
TODO: Document how current engine handles these
```

**Trust System Approach**:
```
TODO: Define our approach
```

---

### 5. Inline vs File Scripts
**Question**: How does the guidebook distinguish between inline commands and file references?

**Examples to Analyze**:
```yaml
signals:
  - name: "inline_example"
    command: "npm test"
  - name: "file_example"  
    command: "./scripts/check.sh"
  - name: "complex_example"
    command: "python ~/analyzer.py --flag"
```

**Current Parsing Logic**:

1. **Signals** (always use `sh -c`):
   ```rust
   // All signals executed as:
   Command::new("sh").arg("-c").arg(&signal.command)
   ```

2. **Actions** (heuristic detection):
   ```rust
   let is_script = (command.starts_with('/') || command.starts_with("./")) 
       && !command.contains("&&") 
       && !command.contains("||") 
       && !command.contains(';')
       && !command.contains('|')
       && !command.contains('>');
   ```

3. **Convention-based discovery**:
   - Signals: Any file in `.cupcake/signals/` becomes a signal
   - Actions: Any file in `.cupcake/actions/` becomes an action
   - Both stored as file paths in the command field

**Trust Handling Strategy**:

```rust
enum ScriptReference {
    /// Pure inline command: "npm test", "echo hello"
    Inline(String),
    
    /// File script: "./check.sh", "/usr/bin/validate"
    File(PathBuf),
    
    /// Complex: "python script.py", "node build.js --prod"
    Complex {
        interpreter: String,
        script_path: PathBuf,
        args: Vec<String>,
    }
}

impl ScriptReference {
    fn parse(command: &str) -> Self {
        let parts: Vec<&str> = command.split_whitespace().collect();
        
        // Direct file execution
        if command.starts_with('/') || command.starts_with("./") {
            return ScriptReference::File(PathBuf::from(command));
        }
        
        // Check for interpreter + script pattern
        if parts.len() >= 2 {
            match parts[0] {
                "python" | "python3" | "node" | "ruby" | "perl" | "bash" | "sh" => {
                    if !parts[1].starts_with('-') && Path::new(parts[1]).exists() {
                        return ScriptReference::Complex {
                            interpreter: parts[0].to_string(),
                            script_path: PathBuf::from(parts[1]),
                            args: parts[2..].iter().map(|s| s.to_string()).collect(),
                        };
                    }
                }
                _ => {}
            }
        }
        
        // Default: inline command
        ScriptReference::Inline(command.to_string())
    }
    
    fn compute_hash(&self) -> Result<String> {
        match self {
            ScriptReference::Inline(cmd) => {
                // Hash the command string itself
                Ok(sha256_string(cmd))
            },
            ScriptReference::File(path) => {
                // Hash file contents
                sha256_file(path)
            },
            ScriptReference::Complex { script_path, .. } => {
                // Hash the script file, ignore interpreter/args
                sha256_file(script_path)
            }
        }
    }
}
```

---

### 6. Trust Initialization Performance
**Question**: What's the startup overhead of loading and verifying the trust manifest?

**Measurements Needed**:
- [ ] JSON parsing time for typical manifest
- [ ] HMAC verification time
- [ ] Impact on `cupcake eval` cold start
- [ ] Impact on repeated evaluations

**Acceptable Thresholds**:
- Cold start overhead: <50ms
- Warm verification: <5ms

**Optimization Opportunities**:
```
TODO: Document based on measurements
```

---

### 7. Trust State Management
**Question**: How do we handle trust state transitions cleanly?

**State Transitions**:
```
Not Initialized -> Trust Enabled
Trust Enabled -> Trust Disabled
Trust Disabled -> Trust Enabled
Trust Enabled -> Trust Updated
```

**Questions**:
- Where is state persisted?
- How do we handle incomplete transitions?
- What about concurrent operations?

**State Machine Design**:
```
TODO: Document state machine
```

---

### 8. Error Handling Integration
**Question**: How does the current engine handle errors? How should trust errors fit in?

**Current Error Patterns**:
- Uses `anyhow::Result` for all error handling
- Context added via `.context("message")?`
- Errors logged via `tracing::{error!, warn!}`
- User-facing errors printed to stderr in main.rs

**Trust Error Strategy**:

Trust violations should be special - they're security events, not just errors. Need clear, actionable messages.

**Error Type Design**:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrustError {
    #[error("Trust not initialized. Run 'cupcake trust init' to enable script integrity verification")]
    NotInitialized,
    
    #[error("Script integrity violation: {path}\nExpected: {expected}\nActual: {actual}\n\nRun 'cupcake trust update' to approve this change")]
    ScriptModified {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    
    #[error("Trust manifest corrupted (HMAC verification failed)\nThis is a critical security event")]
    ManifestTampered,
    
    #[error("Script not in trust manifest: {path}\nRun 'cupcake trust update' to add this script")]
    ScriptNotTrusted {
        path: PathBuf,
    },
    
    #[error("Failed to read script for verification: {path}")]
    ScriptNotFound {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

// Convert to anyhow::Error for consistency
impl From<TrustError> for anyhow::Error {
    fn from(err: TrustError) -> Self {
        anyhow::anyhow!(err)
    }
}
```

**Integration Pattern**:
```rust
// In engine
match self.trust_verifier.verify_script(&command) {
    Ok(()) => { /* proceed */ },
    Err(TrustError::ScriptModified { .. }) => {
        // Log security event
        error!("SECURITY: {}", err);
        // Bubble up with context
        return Err(err.into());
    },
    Err(e) => return Err(e.into()),
}
```

---

### 9. Testing Strategy
**Question**: How do we test trust violations without modifying real files?

**Test Approaches**:
- [ ] Mock filesystem
- [ ] Test fixtures with known hashes
- [ ] In-memory script content
- [ ] Snapshot testing

**Chosen Approach**:
```
TODO: Document testing strategy
```

---

### 10. Version Control Implications
**Question**: Should .trust be in version control or .gitignore?

**Considerations**:

**Version Controlled** (in git):
- ✅ Team consistency
- ✅ Audit trail
- ❌ Merge conflicts
- ❌ Platform-specific paths

**Git Ignored**:
- ✅ No merge conflicts
- ✅ Platform independent
- ❌ Each dev must init trust
- ❌ No shared trust state

**Recommendation**:
```
TODO: Make recommendation with rationale
```

---

### 11. Concurrent Modification Handling
**Question**: What happens if scripts are modified during `cupcake trust update`?

**Race Conditions**:
- Script modified between hash and manifest write
- Multiple trust update commands
- Script deleted during update

**Mitigation Strategies**:
- [ ] File locking?
- [ ] Atomic operations?
- [ ] Retry logic?

**Chosen Approach**:
```
TODO: Document approach
```

---

### 12. User Experience Flow
**Question**: How prominent should trust notifications be? When do we show them?

**Notification Points**:
- [ ] First run without trust
- [ ] Every run without trust?
- [ ] After trust violation
- [ ] After successful trust verification?

**User Research Needed**:
- What's annoying vs helpful?
- How do other tools handle this?

**UX Decision**:
```
TODO: Document UX approach
```

---

## Architecture Decisions

Based on discoveries above, key architectural decisions:

### Decision 1: Trust Module Location
**Options Considered**:
1. Part of engine module - Too coupled
2. Separate trust module - Clean separation ✓
3. Part of harness - Wrong layer

**Decision**: Create `src/trust/` module
```
src/
  trust/
    mod.rs          # Public API
    manifest.rs     # TrustManifest struct and I/O
    verifier.rs     # Verification logic
    hasher.rs       # Hashing utilities
    cli.rs          # CLI command implementations
```
**Rationale**: Clean separation of concerns, testability, potential for reuse

### Decision 2: Integration Pattern
**Options Considered**:
1. Decorator pattern - Too complex for 3 integration points
2. Direct integration - Simple and clear ✓
3. Middleware approach - Over-engineered

**Decision**: Direct integration with Option<TrustVerifier>
```rust
// In Engine
trust_verifier: Option<TrustVerifier>,

// At each execution point
if let Some(trust) = &self.trust_verifier {
    trust.verify_script(&command)?;
}
```
**Rationale**: Consistent with existing patterns, minimal overhead, clear flow

### Decision 3: Manifest Storage Format
**Options Considered**:
1. JSON - Human readable, serde support ✓
2. TOML - Nice for config, but nested structure awkward
3. Binary format - Not debuggable
4. SQLite - Overkill for key-value storage

**Decision**: JSON with pretty printing
```json
{
  "version": 1,
  "timestamp": "2024-01-20T10:00:00Z",
  "scripts": { ... },
  "hmac": "..."
}
```
**Rationale**: Debuggable, version control friendly, easy to inspect

### Decision 4: Script Resolution Strategy
**Decision**: Parse commands into ScriptReference enum
- Handles inline, file, and complex commands
- Resolves paths at manifest creation time
- Stores both original and resolved paths

**Rationale**: Robust handling of all command types users might have

### Decision 5: Trust State Management
**Decision**: Trust state determined by manifest presence
- Manifest exists = trust enabled
- No manifest = trust disabled  
- No separate enable/disable state file

**Rationale**: Simple, no state synchronization issues

### Decision 6: Error Handling
**Decision**: Custom TrustError type with thiserror
- Clear, actionable error messages
- Security events logged specially
- Converts to anyhow::Error for compatibility

**Rationale**: Better UX, security visibility

---

## Performance Budget

Based on discoveries, our performance budget is:

| Operation | Budget | Actual | Status |
|-----------|--------|--------|---------|
| Trust init (cold) | <100ms | TBD | ⏳ |
| Manifest load | <10ms | TBD | ⏳ |
| Single script verify | <1ms | TBD | ⏳ |
| Trust update | <500ms | TBD | ⏳ |
| Disabled overhead | 0ms | TBD | ⏳ |

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Performance regression | Medium | High | Benchmark in CI |
| Complex edge cases | High | Medium | Comprehensive tests |
| Poor UX adoption | Medium | High | User feedback loop |
| Security bypass | Low | Critical | Security audit |

---

## Implementation Priority

Based on discoveries, refined implementation order:

### Phase 1: Core Trust Module (High Priority)
1. Create `src/trust/` module structure
2. Implement `ScriptReference` enum and parsing
3. Implement `TrustManifest` struct with serde
4. Implement SHA-256 hashing utilities
5. Implement HMAC signing with derived keys
6. Create `TrustError` type with thiserror

### Phase 2: Verification Logic (High Priority)
1. Implement `TrustVerifier` struct
2. Add manifest loading and HMAC verification
3. Implement script verification logic
4. Add path resolution and normalization
5. Handle all three script types (inline/file/complex)

### Phase 3: Engine Integration (Medium Priority)
1. Add `Option<TrustVerifier>` to Engine
2. Integrate at `execute_signal` point
3. Integrate at `execute_single_action` point
4. Integrate at convention-based discovery
5. Add trust initialization to Engine::new
6. Add startup notifications for trust status

### Phase 4: CLI Commands (Medium Priority)
1. Implement `cupcake trust init`
2. Implement `cupcake trust update` with diff display
3. Implement `cupcake trust verify`
4. Implement `cupcake trust list`
5. Add trust commands to main CLI parser

### Phase 5: Testing & Polish (Low Priority)
1. Unit tests for ScriptReference parsing
2. Unit tests for manifest serialization
3. Integration tests with mock filesystem
4. Performance benchmarks
5. Documentation updates
6. Example trust workflows

---

## Open Questions

Questions that need team/user input:

1. Should enterprise features be considered now or later?
2. What's the acceptable performance overhead?
3. How verbose should trust notifications be?
4. Should we support partial trust (some scripts trusted, some not)?

---

## Next Steps

After completing discovery:

1. [ ] Review findings with team
2. [ ] Update implementation plan
3. [ ] Create detailed technical design
4. [ ] Begin implementation with highest-risk items first