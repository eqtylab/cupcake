### Proposal: A Scalable and Composable Policy Format

The initial `toml` format was a successful MVP, proving the core value of the Cupcake engine. However, its flat structure and verbosity present challenges to scalability and maintainability—a problem common to monolithic configuration files, often referred to as the "DevSecOps YAML meme."

To address this, we will evolve the policy format by adopting two core principles from modern Infrastructure-as-Code (IaC) tooling: **modularity** and **composition**.

The new format will be based on **YAML** for its balance of human readability and a mature, robust tooling ecosystem. The core idea is to allow policies to be defined across multiple files, organized by domain, and then composed into a single configuration at runtime. This requires a root configuration file that acts as an entrypoint and a clear directory structure for the policy files themselves.

### The `guardrails/` Convention

To establish a clear, industry-standard practice for "Agent Governance as Code," all configuration will reside in a top-level `guardrails/` directory. This name describes the _purpose_ of the code within it: to provide safe, effective boundaries for AI agents.

```
your-project/
├── .github/
├── src/
├── tests/
│
└── guardrails/
    │
    ├── cupcake.yaml         # <-- The root entrypoint. Clean and uncluttered.
    │
    └── policies/            # <-- A dedicated subdirectory for all policy files.
        ├── 00-base.yaml
        ├── 10-security.yaml
        ├── 20-backend.yaml
        ├── 21-frontend.yaml
        └── 99-temp.yaml
```

### 1. The Root `cupcake.yaml` File

The root `cupcake.yaml` file is the master configuration for the Cupcake engine. Its primary job is to define global settings and to **import** the policy files from the `policies/` directory using glob patterns.

```yaml
# guardrails/cupcake.yaml

# This file defines global settings and orchestrates the import of all policies.
settings:
  audit_logging: true
  debug_mode: false

# The 'imports' key uses glob patterns to discover and load policy files.
# This allows new policy sets to be added without modifying the root config.
imports:
  - "policies/*.yaml"
```

This pattern automatically discovers all `.yaml` files within the `policies/` directory. To ensure a predictable merge order, it is standard practice to use numbered prefixes (e.g., `00-`, `10-`) on the policy filenames.

### 2. The Policy Fragment Files

Each file in the `policies/` directory is a "Grouped Map" YAML file, containing only the policies relevant to its specific domain. This structure groups policies by `HookEvent` and then by `matcher`, eliminating repetition.

**Example: `policies/10-security.yaml`**
This file could be owned by the security team, allowing them to manage security rules without creating merge conflicts with other teams.

```yaml
# policies/10-security.yaml
# Contains only security-related policies.

PreToolUse:
  "Bash":
    - name: Block Dangerous Commands (rm, dd)
      conditions:
        - type: pattern
          field: tool_input.command
          regex: "^(rm|dd|format)\\s"
      action:
        type: block_with_feedback
        feedback_message: "🚫 Dangerous command blocked for safety!"

    - name: Prevent Leaking API Keys
      conditions:
        - type: pattern
          field: tool_input.command
          # A regex to catch common API key patterns being echoed or printed
          regex: "echo .*SK-[a-zA-Z0-9]{20,}"
      action:
        type: block_with_feedback
        feedback_message: "Potential API key leak detected. Operation blocked."
```

**Example: `policies/21-frontend.yaml`**
This file could be owned by the frontend team.

```yaml
# policies/21-frontend.yaml
# Contains only frontend-related policies.

PostToolUse:
  "Write|Edit":
    - name: Suggest Prettier for TS/JS files
      conditions:
        - type: pattern
          field: tool_input.file_path
          regex: "\\.(ts|js)x?$"
      action:
        type: provide_feedback
        message: "TypeScript file modified. Remember to run `npm run format`."
```

### 3. Composition Logic and Validation

When `cupcake run` is executed, it performs the following steps:

1.  It finds and parses the root `guardrails/cupcake.yaml`.
2.  It iterates through the `imports` list, resolving each glob pattern to a list of files.
3.  It parses each discovered policy fragment file in the determined order.
4.  It performs a **deep merge** of each file into a single, in-memory policy map.
5.  It validates the final composed structure.

**Merge and Validation Strategy:**

- **Concatenation:** If `10-security.yaml` defines policies under `PreToolUse.Bash` and `20-backend.yaml` also defines policies under `PreToolUse.Bash`, the lists of policies are **concatenated**, not replaced.
- **Uniqueness:** Policy names (the `name` field within each policy definition) **must be unique** across all imported files. The engine will raise a validation error on encountering a duplicate name to prevent accidental overrides and ensure clear audit trails.

### Benefits of this Approach

This model directly addresses the challenges of managing policies at scale and provides a professional, first-class developer experience.

1.  **Clear Ownership:** The security team owns `10-security.yaml`, the frontend team owns `21-frontend.yaml`. This aligns with how large engineering organizations work and drastically reduces merge conflicts.
2.  **Scalability:** The system can handle thousands of policies. Finding a specific rule is as simple as navigating to the correct domain-specific file.
3.  **Composability:** This pattern allows for powerful environment-specific configurations. A project could have a `cupcake.dev.yaml` and a `cupcake.ci.yaml`, each importing a different combination of policy sets.
4.  **Maintainability:** Each file is small, focused, and easier to reason about. The root `cupcake.yaml` provides a high-level table of contents for the entire policy set.
5.  **IDE Integration & Typed Guarantees:** A formal **JSON Schema** will be provided for the `cupcake.yaml` format. This enables rich IDE support, providing developers with auto-completion, inline documentation, and real-time validation, catching errors before runtime.
