### **`CUPCAKE_DESIGN_QA.md`**

**Project:** Cupcake - Policy Enforcement Engine for Claude Code
**Status:** Finalizing V1 Design based on Q&A

This document captures the key design questions and the decisions made. It will serve as a guide for implementation.

### **1. How to handle complex, multi-step rules?**

_(e.g., "if you edit `file.xyz`, you must first read `filexyz.md`")_

- **Decision:** We will support stateful rules from V1. This is a core requirement for many practical policies.
- **Implementation Rationale:** While Claude Code provides its own state via the session transcript, parsing this potentially large and complex file on every hook would be too slow and brittle. Instead, Cupcake will maintain its own lean, performant state file to guarantee sub-100ms performance and reliability.
- **Implementation Details:**
  - Cupcake will manage a session-specific state file: `.cupcake/state/<session_id>.json`. The `session_id` is provided in every hook's input payload.
  - Cupcake automatically tracks ALL tool usage (Read, Write, Edit, Bash, etc.) without needing explicit policies to record these events
  - This file will act as a simple append-only log. For example: `{ "timestamp": "...", "tool": "Read", "success": true, "input": { "file_path": "filexyz.md" } }`
  - Custom business events can be recorded via `update_state` action for more complex workflows
  - The policy schema will support state-aware conditions.
  - **Example Policy (simplified - no tracking policy needed):**
    ```toml
    [[policy]]
    name = "Ensure context is read before editing"
    hook_event = "PreToolUse"
    matcher = "Edit|Write"
    conditions = [
      { type = "filepath_regex", value = "file\\.xyz" },
      { type = "state_missing", tool = "Read", path = "filexyz.md" }
    ]
    action = {
      type = "block_with_feedback",
      feedback_message = "Policy Violation: You must read filexyz.md before editing file.xyz."
    }
    ```

### **2. How to handle policy conflicts?**

- **Decision:** We will use a "Two-Pass Aggregation" model that collects all feedback while respecting critical rules.
- **Implementation:**
  - **Loading Order:** Policies are loaded into a single ordered list:
    1.  **Project Policies** (`./cupcake.toml`) - First (mirrors `./CLAUDE.md`)
    2.  **User Policies** (`~/.claude/cupcake.toml`) - Appended (mirrors `~/.claude/CLAUDE.md`)
  - **Two-Pass Evaluation:**
    - **Pass 1:** Iterate through entire list collecting ALL feedback from matching policies
    - **Pass 2:** Re-iterate to find FIRST hard action (block/approve) and stop
  - **Benefits:**
    - Style rules naturally aggregate (no conflicts)
    - Users see all feedback in one response
    - Project blocks can't be overridden by user preferences
  - The `cupcake init` meta-prompt will instruct Claude to be aware of this model and help resolve true conflicts during setup.

### **3. How to manage `.claude/settings.json` without clobbering user-defined hooks?**

- **Decision:** The `cupcake sync` command will perform a safe read-modify-write cycle.
- **Implementation:**
  1.  The Rust binary will use `serde_json` to read and deserialize the entire `.claude/settings.json` file into a Rust struct.
  2.  It will programmatically navigate to the `hooks` section and add or update only the specific entries for Cupcake (e.g., `command = "cupcake run ..."`), leaving all other settings and hooks untouched.
  3.  It will serialize the modified Rust struct back into JSON and overwrite the file. This is a robust and safe method.

### **4. How to implement complex toolchain assurance?**

_(e.g., running a linter or test suite)_

- **Decision:** We will use a `run_command` action, with Cupcake managing the command execution and feedback loop. The hook configuration in `settings.json` remains simple.
- **Implementation:**
  - The `settings.json` file will only contain a simple, single-entry-point hook.
  - The complexity lives in `cupcake.toml`.
  - **Example Policy:**
    ```toml
    [[policy]]
    name = "Run tests after code changes"
    hook_event = "PostToolUse"
    matcher = "Write|Edit"
    conditions = [ { type = "filepath_regex", value = "\\.rs$" } ] # Only on Rust files
    action = {
      type = "run_command",
      command = "cargo test",
      on_failure_feedback = "Tests failed after your edit. Please fix them. Failing tests:\n{{stderr}}"
    }
    ```
  - When this triggers, `cupcake run` executes `cargo test`. If it fails (non-zero exit code), Cupcake captures `stderr`, formats it into the feedback message, and sends a blocking response back to Claude.

### **5. How expressive should the policy language be?**

- **Decision:** The policy language must be highly expressive. We will start with a core set of powerful primitives and leverage Claude's translation ability to map natural language to them.
- **Implementation - V1 Policy Primitives:**
  | Natural Language Rule | `cupcake.toml` Implementation |
  | :--- | :--- |
  | "Can't delete file `xyz`" | `hook: PreToolUse`, `matcher: Bash`, `condition: command_regex = "rm.*xyz"`, `action: block` |
  | "Must use `<Button>` not `<button>`" | `hook: PreToolUse`, `matcher: Write\|Edit`, `condition: file_content_regex = "<button"`, `action: block` |
  | "Tests must pass before committing" | `hook: PreToolUse`, `matcher: Bash`, `condition: command_regex = "git commit"`, `action: run_command = "npm test"` |
  | "Reusable components go in `components/`" | `hook: PreToolUse`, `matcher: Write`, `condition: filepath_regex = "^(?!components/)"`, `action: block` |
  | "Notify me when a new library is installed" | `hook: PostToolUse`, `matcher: Bash`, `condition: command_regex = "npm install"`, `action: run_command = "notify_script.sh '{{command}}'"` |

### **6. Should we support statefulness from the start?**

- **Decision:** Yes. This is a core requirement. See Q1 for implementation details.

### **7. How do we ensure sub-100ms performance?**

- **Decision:** We will use a file-based caching strategy to eliminate parsing overhead on every run.
- **Implementation:**
  1.  The `cupcake run` command will compare the last-modified timestamp of `cupcake.toml` with a `.cupcake/policy.cache` file.
  2.  If `cupcake.toml` is unchanged, Cupcake will load the pre-parsed, binary-serialized policy set directly from the cache file using a fast deserializer like `bincode`. This will be nearly instantaneous.
  3.  If `cupcake.toml` has been modified, Cupcake will perform a full parse and then write the new policy set to the cache file for subsequent runs.

### **8. How are policies from multiple `CLAUDE.md` sources merged and prioritized?**

- **Decision:** Merging and prioritization will happen **once** during the interactive `cupcake init` process, not at runtime.
- **Implementation:**
  1.  **Discovery:** `cupcake init` recursively finds all `CLAUDE.md` files (project-level, user-level, and via `@imports`).
  2.  **AI-Assisted Aggregation:** It provides the content of all found files to a single Claude session. The meta-prompt instructs Claude to act as a policy administrator, translating all rules into a single, coherent `cupcake.toml`.
  3.  **Conflict Resolution:** The prompt explicitly asks Claude to identify duplicates or conflicts and to work interactively with the user to resolve them.
  4.  **Single Source of Truth:** The output is one final `cupcake.toml`. At runtime, `cupcake run` only ever needs to consult this single, pre-resolved file, ensuring maximum performance.

### **9. How do we handle validation and error recovery during `init`?**

- **Decision:** We will create a self-correcting loop during initialization.
- **Implementation:**
  1.  `cupcake init` invokes Claude to generate a temporary `cupcake.toml.tmp` file.
  2.  It then automatically runs an internal `cupcake validate` command against the temp file.
  3.  If validation fails, Cupcake captures the error, re-prompts the _same Claude session_ with the invalid file and the error message, and instructs it to fix its own output.
  4.  This loop continues until a valid file is produced, which is then presented to the user for final approval.

### **10. Should `cupcake.toml` include a version field?**

- **Decision:** Yes. It is a low-cost, high-value feature for future compatibility.
- **Implementation:** The generated `cupcake.toml` will include `policy_schema_version = "1.0"` at the top.

### **11. Should we use a binary or a daemon?**

- **Decision:** We will use a stateless binary.
- **Implementation:** Performance concerns are fully addressed by the caching strategy (Q7). The binary approach is simpler, more secure (no long-running processes), and more reliable, aligning perfectly with the hook execution model.

### **12. Should Cupcake provide an audit trail?**

- **Decision:** Yes, this is a valuable feature for enterprise compliance and debugging.
- **Implementation:**
  - We will add a setting in `cupcake.toml`: `[settings] audit_logging = true` (defaults to `false`).
  - When enabled, `cupcake run` will append a structured JSON line to `.cupcake/audit.log` for every policy decision it makes (approve, block, or command run).
