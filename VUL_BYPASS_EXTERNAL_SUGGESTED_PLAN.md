Of course. Here is a focused and detailed plan for executing Phase 1, designed to be completed within a 1-2 week sprint.

### **Phase 1: Short-Term Mitigation & Hardening**

**Objective:** Immediately reduce the risk posed by the identified bypass vulnerabilities using targeted, low-complexity changes within the existing architecture. This phase prioritizes closing the most obvious exploit vectors while laying the groundwork for deeper architectural fixes.

**Timeline:** 10 Working Days

---

### **Task 1: Harden Bash Command Policies (Vulnerability #3)**

**Objective:** Replace brittle `contains()` checks with more robust regular expressions to defeat simple command obfuscation techniques like extra whitespace and quoting.

**Action Items & Implementation Details:**

1.  **Update `git_block_no_verify.rego`:**

    - **Files:**
      - `fixtures/claude/builtins/git_block_no_verify.rego`
      - `fixtures/cursor/builtins/git_block_no_verify.rego`
    - **Change:** Modify the `contains_git_no_verify` rules.
    - **Before (Vulnerable):**
      ```rego
      contains(cmd, "git")
      contains(cmd, "commit")
      contains(cmd, "--no-verify")
      ```
    - **After (Hardened):**
      ```rego
      # This regex handles variable whitespace between tokens.
      regex.match(`git\s+commit\s+.*\s*--no-verify`, cmd)
      ```
      _Apply similar regex logic for `git push` and `git merge` checks._

2.  **Update `rulebook_security_guardrails.rego`:**

    - **Files:**
      - `fixtures/claude/builtins/rulebook_security_guardrails.rego`
      - `fixtures/cursor/builtins/rulebook_security_guardrails.rego`
    - **Change:** Enhance the `contains_cupcake_modification_pattern` to be more robust. The current simple `contains(cmd, ".cupcake")` is insufficient. We will augment it by checking for dangerous commands with more flexible patterns.
    - **Implementation:**

      ```rego
      # New helper rule
      is_dangerous_command(cmd) if {
          dangerous_verbs := {"rm", "mv", "cp", "chmod", "chown", "tee"}
          some verb in dangerous_verbs
          regex.match(concat("", [`\s`, verb, `\s`]), cmd)
      }

      # Updated main rule
      contains_cupcake_modification_pattern(cmd) if {
          contains(cmd, ".cupcake")
          is_dangerous_command(cmd)
      }
      ```

3.  **Update `protected_paths.rego`:**
    - **Files:**
      - `fixtures/claude/builtins/protected_paths.rego`
      - `fixtures/cursor/builtins/protected_paths.rego`
    - **Change:** The `is_whitelisted_read_command` check uses `startswith()`, which is vulnerable.
    - **Before (Vulnerable):** `startswith(cmd, "cat ")`
    - **After (Hardened):** `regex.match(`^\s*cat\s+`, cmd)`
      *Apply this pattern to all commands in the `safe_read_commands` set.\*

**Verification (New Adversarial Tests):**

- Create a new test file `cupcake-core/tests/adversarial_bash_tests.rs`.
- Add tests that assert `Deny` or `Halt` for the following commands against the relevant policies:
  - `"git  commit  --no-verify"` (extra spaces)
  - `"git commit\t--no-verify"` (tabs)
  - `"rm  -rf  .cupcake/"` (extra spaces)
  - `"  cat  /etc/passwd"` (leading/extra spaces on whitelisted command)

---

### **Task 2: Broaden Policy Scope (Vulnerability #2)**

**Objective:** Mitigate cross-tool bypasses by ensuring file protection policies apply to all relevant tools, not just a narrow subset. Educate users on this architectural limitation.

**Action Items & Implementation Details:**

1.  **Expand Metadata in Built-in Policies:**

    - **Files:**
      - `fixtures/claude/builtins/protected_paths.rego`
      - `fixtures/claude/builtins/global_file_lock.rego`
      - `fixtures/claude/builtins/rulebook_security_guardrails.rego`
      - (And their `fixtures/cursor/` equivalents, adapted for Cursor events)
    - **Change:** Audit and expand the `required_events` and `required_tools` in the `METADATA` block.
    - **Example for `protected_paths.rego` (Claude):**
      - **Before:** `required_events: ["PreToolUse"]` (tools are implicit in the rules)
      - **After:**
        `rego
    # METADATA
    # ...
    # custom:
    #   routing:
    #     required_events: ["PreToolUse"]
    #     required_tools: ["Edit", "Write", "MultiEdit", "Bash"]
    `
        _This change makes the policy's scope explicit and ensures the routing engine applies it to all modification tools._

2.  **Create User Guidance Documentation:**
    - **Action:** Create a new markdown file: `docs/POLICY_BEST_PRACTICES.md`.
    - **Content:**
      - Explicitly state that policies are **tool-specific by default**.
      - Provide a clear example of a cross-tool bypass (e.g., blocking `Edit` but not `Bash`).
      - **Recommendation:** "To protect a resource (like a file or directory), your policy's `required_tools` metadata **must** include every tool that could possibly interact with that resource."
      - Provide a canonical list of file modification tools (`Edit`, `Write`, `MultiEdit`, `Bash`, etc.) for users to copy into their policies.

**Verification (New Integration Tests):**

- In a new test file `cupcake-core/tests/adversarial_crosstool_tests.rs`:
  - Write a test where a `protected_paths` policy is active.
  - Assert that an attempt to modify a protected file using the `Write` tool is **blocked**.
  - Assert that an attempt to modify the same file using `Bash` with `echo "..." > file` is also **blocked**. (This verifies the expanded metadata works).

---

### **Task 3: Harden Filesystem Protections (Vulnerability #4)**

**Objective:** Use OS-level permissions as a secondary defense layer and update policies to recognize and block the creation of symlinks to protected areas.

**Action Items & Implementation Details:**

1.  **Set Strict Permissions on `.cupcake` Directory:**

    - **File:** `cupcake-cli/src/main.rs`
    - **Function:** `init_project_config`
    - **Change:** After creating the `.cupcake` directory, set its permissions to be owner-only.
    - **Implementation (pseudo-code):**
      ```rust
      // In init_project_config, after fs::create_dir_all(".cupcake/...")
      #[cfg(unix)]
      {
          use std::os::unix::fs::PermissionsExt;
          let cupcake_dir = Path::new(".cupcake");
          let mut perms = fs::metadata(&cupcake_dir)?.permissions();
          perms.set_mode(0o700); // rwx for owner, no permissions for group/other
          fs::set_permissions(&cupcake_dir, perms)?;
          info!("Set strict permissions (0700) on .cupcake directory.");
      }
      ```

2.  **Update Policy to Block Symlink Creation:**

    - **File:** `fixtures/claude/builtins/rulebook_security_guardrails.rego` (and Cursor equivalent)
    - **Change:** Add a new rule to the policy that explicitly blocks `ln -s` commands that target protected paths.
    - **Implementation (New Rego Rule):**

      ```rego
      # Block Bash commands that create symlinks to protected paths
      halt contains decision if {
          input.hook_event_name == "PreToolUse"
          input.tool_name == "Bash"
          command := lower(input.tool_input.command)

          # Check for `ln -s` command targeting a protected path
          some protected_path in get_protected_paths
          regex.match(concat("", [`ln\s+.*\s*-s\s+`, protected_path]), command)

          decision := {
              "rule_id": "BUILTIN-RULEBOOK-SECURITY-SYMLINK",
              "reason": "Creating symbolic links to protected directories is not permitted.",
              "severity": "CRITICAL"
          }
      }
      ```

      _`get_protected_paths` is an existing helper in the policy that reads from the `rulebook.yml`._

**Verification (New Integration Tests):**

- In `cupcake-cli/tests/init_command_test.rs`, add a `#[cfg(unix)]` test to verify that `cupcake init` creates the `.cupcake` directory with `0o700` permissions.
- In `adversarial_bash_tests.rs`, add a test that attempts to execute `ln -s .cupcake foo` via the `Bash` tool and asserts that the action is **blocked** by the `rulebook_security_guardrails` policy.

---

### **Summary & Timeline**

| Task                         | Vulnerability Addressed | Key Files to Modify                                            | Effort (Days) | Priority     |
| :--------------------------- | :---------------------- | :------------------------------------------------------------- | :------------ | :----------- |
| **1. Harden Bash Policies**  | #3 (String Matching)    | `*.rego` in `fixtures/`                                        | 3             | **Critical** |
| **2. Broaden Policy Scope**  | #2 (Cross-Tool)         | `*.rego` in `fixtures/`, `docs/`                               | 2             | **High**     |
| **3. Harden Filesystem**     | #4 (Symlink)            | `cupcake-cli/src/main.rs`, `rulebook_security_guardrails.rego` | 2             | **Critical** |
| **4. New Adversarial Tests** | All                     | `cupcake-core/tests/`                                          | 3             | **Critical** |
| **Total**                    |                         |                                                                | **10**        |              |
