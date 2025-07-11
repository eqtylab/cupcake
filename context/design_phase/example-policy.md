You are absolutely right. My previous answer was a patch on the "first match wins" model, but it failed to address your core, valid concerns about composability and scalability. Forcing related rules into a single, monolithic policy block is a design flaw.

Thank you for pushing back. This is a critical insight.

Let's scrap that idea and design a system that is _truly_ composable and can handle 20+ independent checks on a single file gracefully.

The solution is to evolve the evaluation model from a simple "first match wins" to a more sophisticated **multi-pass evaluation strategy**. The goal is to separate policies that provide feedback from policies that represent a hard, non-negotiable block.

### The New Evaluation Model: Two-Pass Aggregation

Here's how `cupcake run` will now work. For any given event (like a `PreToolUse` on a file edit), it will perform two passes over the entire ordered policy list (Enterprise -> Project -> User).

#### **Pass 1: Collect All Feedback (The "Soft" Pass)**

In this pass, Cupcake iterates through **all** policies and looks for any that are designed to provide non-blocking feedback.

- It **does not stop** on the first match.
- It collects the feedback messages from every matching "soft" policy.
- These are rules where a violation is undesirable but not critical enough to halt everything.

#### **Pass 2: Check for Hard Blocks (The "Hard" Pass)**

After collecting all possible feedback, Cupcake makes a second pass to check for critical, show-stopping violations.

- This pass uses the original **"first match wins"** logic.
- The first "hard" policy that triggers will halt the entire process and its feedback will override everything else.
- These are rules like "Tests must pass before commit" or "Do not edit this locked file."

### The Final Decision Logic

After the two passes, Cupcake makes a final decision:

1.  **If a hard block was triggered in Pass 2:** The operation is blocked AND all collected feedback from both passes is combined into a comprehensive message. (e.g., "Operation blocked due to security violation. Additionally found 2 style issues: Use `<Button>`, Use `<Link>`").
2.  **If no hard block was triggered, but feedback was collected in Pass 1:** Cupcake aggregates all the collected feedback into a single, comprehensive message and sends that to Claude as a block. (e.g., "Found 2 policy violations: Use `<Button>`, Use `<Link>`").
3.  **If no hard block and no feedback was found:** The action is approved, and Claude's workflow continues uninterrupted.

### How This Looks in `cupcake.toml`

To support this, we introduce more expressive `action` types in our policy schema.

```toml
# In your project's cupcake.toml

# --- POLICY 1: A composable, independent "soft" rule ---
[[policy]]
name = "Enforce <Button> component"
hook_event = "PreToolUse"
matcher = "Write|Edit"
conditions = [ { type = "filepath_regex", value = "\\.(jsx|tsx)$" } ]
action = {
  # This is a "soft" rule. It only provides feedback.
  type = "provide_feedback",
  # The pattern to check for in the file content.
  pattern = "<button",
  # The message to add to the feedback list if the pattern is found.
  message = "• Use the custom `<Button>` component instead of a native `<button>` tag."
}

# --- POLICY 2: Another composable, independent "soft" rule ---
[[policy]]
name = "Enforce <Link> component"
hook_event = "PreToolUse"
matcher = "Write|Edit"
conditions = [ { type = "filepath_regex", value = "\\.(jsx|tsx)$" } ]
action = {
  type = "provide_feedback",
  pattern = "<a",
  message = "• Use the custom `<Link>` component instead of a native `<a>` tag."
}

# --- POLICY 3: A "hard" blocking rule from the Enterprise config ---
[[policy]]
name = "Enterprise: Prevent direct edits to generated files"
hook_event = "PreToolUse"
matcher = "Write|Edit"
conditions = [ { type = "filepath_regex", value = "\\.generated\\.ts$" } ]
action = {
  # This is a "hard" rule. It stops everything.
  type = "block_with_feedback",
  message = "Enterprise Policy Violation: Do not edit auto-generated files directly."
}
```

### Walkthrough with the New Model

Imagine Claude tries to edit `src/components/UserProfile.tsx` with content containing both `<button>` and `<a>`.

1.  **Pass 1 (Collect Feedback):**

    - Cupcake checks the `<Button>` policy. **Match.** It adds `"• Use the custom `<Button>`..."` to a temporary feedback list. It **continues**.
    - Cupcake checks the `<Link>` policy. **Match.** It adds `"• Use the custom `<Link>`..."` to the feedback list. It **continues**.
    - Cupcake checks the Enterprise policy. The `action` type is not `provide_feedback`, so it's ignored in this pass.
    - _Result of Pass 1:_ A list containing two feedback messages.

2.  **Pass 2 (Check for Hard Blocks):**

    - Cupcake starts from the top again.
    - It checks the `<Button>` policy. The action is "soft", so it's skipped.
    - It checks the `<Link>` policy. The action is "soft", so it's skipped.
    - It checks the Enterprise policy. The `filepath` does not match (`UserProfile.tsx` != `*.generated.ts`). No match.
    - _Result of Pass 2:_ No hard blocks were triggered.

3.  **Final Decision:**
    - Was a hard block found? **No.**
    - Was any feedback collected? **Yes.**
    - **Outcome:** Cupcake aggregates the two messages from Pass 1 and sends a single, comprehensive block to Claude.

This design is fundamentally better because it is:

- **Composable:** Each rule is a small, independent unit. You can define them in different files (Enterprise, Project, User) and the system will intelligently evaluate them.
- **Scalable:** You can have 20, 50, or 100 `provide_feedback` policies. The system will correctly check all of them and aggregate the results without any change in logic.
- **Hierarchical:** The "hard block" pass still respects the "first match wins" logic, ensuring that critical security and compliance rules always have the final say.
