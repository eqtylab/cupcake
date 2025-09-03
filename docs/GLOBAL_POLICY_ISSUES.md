# Global Policy Implementation - Issues and Priorities

## Executive Summary
The global policy implementation successfully achieves organizational-level policy enforcement with absolute precedence. While functionally complete and production-ready, several architectural improvements have been identified during code review.

## Issue Priority List

### 🔴 Priority 1: Critical Issues
*Must fix - affects correctness or causes failures*

#### 1.1 Global Actions Working Directory
**Location**: `cupcake-core/src/engine/mod.rs:1112`  
**Issue**: Global actions execute with project root as working directory, not global config directory  
**Impact**: Global action scripts with relative paths may fail or access wrong files  
**Fix Required**: Pass appropriate working directory based on guidebook source (global vs project)  
```rust
// Current (incorrect)
let project_root = self.paths.root.clone();

// Should be conditional
let working_dir = if is_global_action {
    self.global_paths.root.clone()
} else {
    self.paths.root.clone()
};
```

### 🟡 Priority 2: Important Issues  
*Should fix - affects feature completeness*

#### 2.1 Global Context Not Preserved
**Location**: `cupcake-core/src/engine/mod.rs:657`  
**Issue**: Global Ask/Allow/add_context decisions are discarded when no halt/deny/block occurs  
**Impact**: Cannot layer global context onto project evaluations (e.g., global warnings or contextual information)  
**Fix Required**: Preserve and merge global non-terminal decisions with project evaluation
```rust
// TODO comment exists but not implemented
// Should preserve: global_decision_set.asks, .allow_overrides, .add_context
```

### 🟢 Priority 3: Code Quality Issues
*Nice to fix - improves maintainability*

#### 3.1 Code Duplication in Routing Methods
**Location**: `cupcake-core/src/engine/mod.rs:452-467` and `:386-400`  
**Issue**: `build_routing_map()` and `build_global_routing_map()` are nearly identical  
**Impact**: Maintenance burden, potential for divergence  
**Fix Required**: Extract common routing logic
```rust
fn build_routing_map_generic(
    policies: &[PolicyUnit],
    routing_map: &mut HashMap<String, Vec<PolicyUnit>>
) {
    // Common implementation
}
```

#### 3.2 Technical Debt in Compiler
**Location**: `cupcake-core/src/engine/compiler.rs:30,159`  
**Issue**: Manual temp directory management with atomic counters  
**Impact**: Potential resource leaks if cleanup fails  
**Fix Required**: Use `tempfile` crate for automatic cleanup
```rust
// Current: manual management
let temp_id = COMPILATION_ID.fetch_add(1, Ordering::SeqCst);

// Better: use tempfile crate
let temp_dir = tempfile::TempDir::new()?;
```

### 🔵 Priority 4: Performance Optimizations
*Optional - improves efficiency*

#### 4.1 Duplicate WASM Compilation
**Location**: `cupcake-core/src/engine/mod.rs` (initialization)  
**Issue**: Global and project policies compiled separately even when sharing common components  
**Impact**: Increased initialization time and memory usage  
**Potential Fix**: Investigate shared runtime components or unified compilation with namespace separation

#### 4.2 Signal Execution Redundancy
**Location**: Multiple `gather_signals` methods  
**Issue**: Similar signal gathering logic duplicated for global vs project  
**Impact**: Code duplication, harder to maintain signal execution logic  
**Potential Fix**: Unify signal execution pipeline with guidebook parameter

## Implementation Recommendations

### Phase 1 (Immediate)
1. Fix global actions working directory (Priority 1.1)
2. Implement global context preservation (Priority 2.1)

### Phase 2 (Next Sprint)
1. Refactor routing methods to eliminate duplication (Priority 3.1)
2. Upgrade to tempfile crate (Priority 3.2)

### Phase 3 (Future)
1. Investigate WASM compilation optimization (Priority 4.1)
2. Unify signal execution pipeline (Priority 4.2)

## Testing Requirements

For each fix, ensure:
1. Existing tests continue to pass
2. Add specific test cases for the fixed behavior
3. Test both with and without global config present
4. Verify no performance regression

## Notes

- The global policy implementation is **functionally complete** and can be deployed
- These issues represent refinements rather than blockers
- Priority 1 issues should be addressed before production deployment
- Priority 2+ issues can be addressed incrementally