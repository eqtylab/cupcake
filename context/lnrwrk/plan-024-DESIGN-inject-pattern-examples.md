Of course. You've hit on a crucial point: before diving into complex, dynamic logic, a user needs a simple, reliable way to establish a baseline of context. The "always-on" injection is a foundational use case.

Here are the simpler examples you're looking for, followed by the more advanced ones for a complete picture. This shows the progression from simple static configuration to powerful dynamic integration.

---

### **1. Simple "Always-On" Context Injection (The "Global Config" Use Case)**

This is for injecting a static string on every prompt or every session start. In Cupcake's policy format, "always" is achieved by providing an empty `conditions` list (`[]`).

#### **A. Always Inject Context on _Every_ User Prompt**

**Use Case:** You want to remind the agent of core, non-negotiable development principles _every single time_ it receives a prompt, ensuring these rules are always top-of-mind.

```yaml
# In policies/01-global-context.yaml
UserPromptSubmit:
  "*": # The "*" matcher applies this to all prompts.
    - name: "Always Inject Core Dev Principles"
      description: "Injects a static block of core principles on every user prompt."
      # An empty conditions list means this policy ALWAYS runs for the event.
      conditions: []
      action:
        type: "inject_context"
        # 'content' is used for a simple, static string.
        content: |
          [Core Principles]
          - Security First: Always sanitize inputs and parameterize queries.
          - Simplicity: Write clear, readable, and maintainable code. Do not over-engineer.
          - Testing: All new logic must be accompanied by unit tests.
        # It's best practice to suppress this output so it doesn't clutter the user's transcript on every prompt.
        suppress_output: true
```

#### **B. Always Inject Context on _Every_ Session Start**

**Use Case:** You want to provide a one-time "welcome packet" of information to the agent at the beginning of each session, orienting it to the project.

```yaml
# In policies/01-global-context.yaml
SessionStart:
  "*": # The "*" matcher applies this to every session start.
    - name: "Inject Project Welcome Packet"
      description: "On every session start, provide Claude with a project overview."
      conditions: [] # Empty conditions means this always runs.
      action:
        type: "inject_context"
        content: |
          [Project: Cupcake - Session Start]
          Welcome to the Cupcake codebase. You are a Rust-based policy engine for AI agents.
          - To run tests: `cargo test`
          - To validate policies: `cupcake validate`
          - Key architecture files are in `src/engine/` and `src/config/`.
          Please adhere to the project's coding standards.
        # Suppressing this is optional, but keeps the initial startup view clean.
        suppress_output: false # Set to false to see this message on startup.
```

---

### **2. Advanced & Conditional Context Injection**

These examples build on the simple foundation, showing how to inject context _dynamically_ and _conditionally_, which is where Cupcake's true power as an integration harness shines.

#### **A. Dynamic Injection via Command (`from_command`)**

**Use Case:** When the agent is asked about a specific API endpoint, run a script to fetch its documentation and inject it.

```yaml
# In policies/10-dynamic-context.yaml
UserPromptSubmit:
  "*":
    - name: "Inject API Endpoint Documentation"
      description: "If the user asks about an API endpoint, fetch its schema and provide it to Claude."
      conditions:
        - type: "pattern"
          field: "prompt"
          regex: "(?i)api/v1/(\w+)" # Matches prompts like "how does /api/v1/users work?"
      action:
        type: "inject_context"
        # 'from_command' executes a command and injects its stdout.
        from_command:
          spec:
            mode: array
            command: ["./scripts/get-api-spec.sh", "{{prompt}}"]
          on_failure: "continue" # If the script fails, don't block the agent.
        suppress_output: true
```

#### **B. Conditional Static Injection**

**Use Case:** If the user's prompt mentions "commit", but _not_ "test", inject a reminder to run tests. This is more targeted than the "always-on" example.

```yaml
# In policies/10-dynamic-context.yaml
UserPromptSubmit:
  "*":
    - name: "Inject Pre-Commit Test Reminder"
      description: "If a prompt seems to be about committing code, remind the user to run tests."
      conditions:
        # Condition 1: The prompt must mention 'commit'.
        - type: "pattern"
          field: "prompt"
          regex: "(?i)commit"
        # Condition 2: The prompt must NOT mention 'test'.
        - type: "not"
          condition:
            type: "pattern"
            field: "prompt"
            regex: "(?i)test"
      action:
        type: "inject_context"
        content: |
          [Pre-Commit Checklist]
          The user is asking to create a commit. Before you do, please ensure you have run the relevant tests.
        suppress_output: true
```
