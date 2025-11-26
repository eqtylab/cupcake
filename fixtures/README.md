## Fixtures Directory Usage Analysis

The `fixtures/` directory at the project root serves as the **authoritative source of built-in policies and templates** for the Cupcake project. Here's how it's used:

### 1. **Primary Use: `cupcake init` Command** (cupcake-cli/src/main.rs)

The fixtures are embedded at compile-time using `include_str!` macro and deployed when users run `cupcake init`:

```rust
// Template for initial configuration
const RULEBOOK_TEMPLATE: &str = include_str!("../../fixtures/init/base-config.yml");

// Claude Code builtin policies
const CLAUDE_ALWAYS_INJECT_POLICY: &str =
    include_str!("../../fixtures/claude/builtins/claude_code_always_inject_on_prompt.rego");
// ... and all other builtins

// Cursor builtin policies
const CURSOR_GIT_BLOCK_NO_VERIFY_POLICY: &str =
    include_str!("../../fixtures/cursor/builtins/git_block_no_verify.rego");
// ... and all other builtins
```

When a user runs `cupcake init --harness claude` or `cupcake init --harness cursor`, these embedded policies are written to `.cupcake/policies/builtins/` in their project.

### 2. **Directory Structure**

```
fixtures/
├── init/
│   └── base-config.yml           # Template for rulebook.yml configuration
├── claude/
│   ├── builtins/                 # Claude Code-compatible builtin policies
│   │   ├── claude_code_always_inject_on_prompt.rego
│   │   ├── git_pre_check.rego
│   │   ├── post_edit_check.rego
│   │   ├── rulebook_security_guardrails.rego
│   │   ├── protected_paths.rego
│   │   ├── git_block_no_verify.rego
│   │   └── claude_code_enforce_full_file_read.rego
│   └── system/
│       └── evaluate.rego         # Claude system aggregation entrypoint
├── cursor/
│   ├── builtins/                 # Cursor-compatible builtin policies
│   │   └── (same as Claude but adapted for Cursor events)
│   └── system/
│       └── evaluate.rego         # Cursor system aggregation entrypoint
└── global_builtins/              # Global policies (machine-wide)
    ├── claude/
    │   ├── system_protection.rego
    │   ├── sensitive_data_protection.rego
    │   └── cupcake_exec_protection.rego
    └── cursor/
        └── (same policies adapted for Cursor)
```

### 3. **How It Works**

1. **Compile-time Embedding**: All fixture files are embedded into the binary at compile time
2. **Deployment**: When `cupcake init` runs, it:
   - Creates `.cupcake/` directory structure
   - Writes the appropriate builtin policies based on harness type
   - Creates `rulebook.yml` from the template
   - Sets up harness-specific hooks (e.g., `.claude/settings.json` or `~/.cursor/hooks.json`)

3. **Harness-Specific Deployment**:
   - Claude Code gets all 7 builtins + 3 global builtins
   - Cursor gets 6 builtins (no `claude_code_enforce_full_file_read` due to incompatibility) + 3 global builtins

### 4. **Test Usage**

The test suite also references fixtures:

- **Integration tests** use production fixtures via `include_str!`:
  ```rust
  // From builtin_integration.rs
  let post_edit_policy = include_str!("../../fixtures/claude/builtins/post_edit_check.rego");
  ```

- **Test-specific fixtures** in `cupcake-core/tests/fixtures/`:
  - `system_evaluate.rego` - Test version of system aggregation
  - `minimal_policy.rego` - Minimal test policy
  - `global_system_evaluate.rego` - Global namespace test policy

### 5. **Key Design Decisions**

- **Authoritative Source**: Fixtures are the single source of truth for builtin policies
- **Compile-time Safety**: Embedding at compile-time ensures policies are always available
- **Harness Adaptation**: Separate directories for each harness allow event-specific customization
- **No Runtime Dependencies**: Users don't need the fixtures directory after `cupcake init`
- **Version Control**: All builtin policies are versioned with the binary

This design ensures that every Cupcake installation has consistent, tested builtin policies that are appropriate for their chosen AI coding agent harness.