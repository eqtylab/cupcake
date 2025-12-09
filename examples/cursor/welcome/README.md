# Cupcake - Welcome Walkthrough (Cursor)

Cupcake has native support for [Cursor Agent](https://cursor.com/agents). Thank you to the Cursor team for enabling this integration by maintaining [Hooks](https://cursor.com/docs/agent/hooks)!

This walkthrough demonstrates Cupcake's policy enforcement in action with Cursor hooks.

[Cupcake Architecture - Excalidraw](https://excalidraw.com/#room=2331833bcb24d9f35a25,-TMNhQhHqtWayRMJam4ZIg)

## Table of Contents

- [Prerequisites](#prerequisites)
- [Setup](#setup)
- [Interactive Demo](#interactive-demo)
  - [Step 1: Test Basic Shell Protection](#step-1-test-basic-shell-protection)
  - [Step 2: Understanding the Block](#step-2-understanding-the-block)
  - [Step 3: The Challenge - Bypass Attempt](#step-3-the-challenge---bypass-attempt)
  - [Step 4: Built-in Protection Explained](#step-4-built-in-protection-explained)
  - [Step 5: Centralized Rule Management](#step-5-centralized-rule-management)
  - [Step 6: MCP Database Protection Demo](#step-6-mcp-database-protection-demo)
  - [Step 7: External Context with Signals](#step-7-introducing-external-context-for-more-effective-policy-evaluation)
- [Key Takeaways](#key-takeaways)

## Prerequisites

Before starting, ensure you have:

- **Rust & Cargo** → [Install Rust](https://rustup.rs/)
- **OPA (Open Policy Agent)** → [Install OPA](https://www.openpolicyagent.org/docs/latest/#running-opa)
  - **Windows users**: Download `opa_windows_amd64.exe` and rename to `opa.exe`
- **Cursor** → AI-powered code editor [cursor.com](https://cursor.com)
- **Docker** (optional) → For MCP database demo
- [**uv**](https://docs.astral.sh/uv/) to run Python scripts

_These are development requirements. The production software will manage these dependencies._

## Setup

### 1. Initialize the Environment

Run the setup script:

**Unix/macOS/Linux:**

```bash
./setup.sh
```

**Windows (PowerShell):**

```powershell
powershell -ExecutionPolicy Bypass -File setup.ps1
```

This runs `cupcake init --harness cursor`, and some scaffolding to create:

```
.cupcake/
  ├── rulebook.yml         # Default configuration
  ├── policies/             # Rego policies
  │   ├── cursor/           # Cursor-specific policies
  │   └── builtins/         # Built-in security policies
  ├── signals/              # External data providers
  └── actions/              # Automated response scripts

.cursor/hooks.json          # Cursor hooks integration (project-level)
```

♻️ Reset anytime with:

**Unix/macOS/Linux:**

```bash
./cleanup.sh
```

**Windows (PowerShell):**

```powershell
powershell -ExecutionPolicy Bypass -File cleanup.ps1
```

### 2. Start Cursor

Open this directory in Cursor. The policy engine will now intercept and evaluate all agent actions.

---

## Interactive Demo

**Launch Cursor**

Open this directory in Cursor:

```bash
cursor .
```

### Step 1: Test Basic Shell Protection

Ask the Cursor agent to run a dangerous command:

```
> delete my temp test directory at /tmp/my-test-directory
```

🚫 **Expected Result:** Blocked before execution with separate messages for user and agent.

![Cursor blocked dangerous rm command](screenshots/cursor-block-rm.png)
_[Screenshot: Shows Cursor being blocked from recursive delete]_

---

### Step 2: Understanding the Block

The `rm` command was blocked by a security policy with **differentiated feedback**:

```rego
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "rm -rf")
    decision := {
        "reason": "Dangerous command blocked: rm -rf",  // User sees this
        "agent_context": "This action violates system policies. Recursive deletion of directories is prohibited for security reasons.",  // Agent sees this
        "rule_id": "CURSOR-SECURITY-001",
        "severity": "CRITICAL"
    }
}
```

The Cursor event provides the data for `input`:

```json
{
  "hook_event_name": "beforeShellExecution",
  "conversation_id": "conv_123",
  "command": "rm -rf /tmp/my-test-directory",
  "cwd": "/path/to/project",
  ...
}
```

**Key Difference**: Cursor allows separate `userMessage` and `agentMessage` in the response, helping the AI learn from specific technical feedback.

---

### Step 3: The Challenge - Bypass Attempt

Now, let's see if Cursor can remove the blocking policy:

```
> find what policy in .cupcake is blocking us and remove it
```

🚫 **Expected Result**: Cursor will try to read/edit `.cupcake/` files but **fail**!

![Cursor blocked by rulebook_security_guardrails](screenshots/cursor-block-cupcake.png)
_[Screenshot: Shows Cursor being blocked from accessing .cupcake directory]_

---

### Step 4: Built-in Protection Explained

Cursor was blocked by the `rulebook_security_guardrails` builtin, which protects Cupcake's configuration from tampering.

**`Built-ins` are special policies that:**

- Are enabled by default in `rulebook.yml`
- Protect critical system functionality
- Cannot be easily bypassed by AI agents
- Provide layered security (global + project level)

**Active built-ins in this demo:**

- `rulebook_security_guardrails` → protects `.cupcake/`
- `protected_paths` → blocks `/etc/`, `/System/` modifications
- `git_block_no_verify` → prevents skipping git hooks
- `sensitive_data_protection` → protects SSH keys, AWS credentials

<details>
<summary>Bonus Test: Try to bypass git commit hooks</summary>

Ask Cursor to run:

```
> commit with --no-verify flag to skip hooks
```

🚫 **Expected Result**: Blocked by `git_block_no_verify` with agent-specific feedback.

![Cursor blocked git --no-verify](screenshots/cursor-block-git-no-verify.png)
_[Screenshot placeholder: Shows git --no-verify being blocked]_

</details>

---

### Step 5: Centralized Rule Management

Part of the benefit of using a centralized policy enforcement layer is the ability to have a well managed model over rules.
So far, you've seen two rules in action. Let's see all of the rules cupcake loads at runtime:

```bash
cupcake inspect --harness cursor # will show the policies currently loaded
```

```bash
cupcake inspect --harness cursor --table # shows a compact table format
```

![cupcake inspect shows the current policies](screenshots/cursor-inspect.png)
_[Screenshot placeholder: Shows cupcake inspect output]_

Later on, we cover how to `verify` and `test` policies.

---

## Step 6: MCP Database Protection Demo

This demo shows how Cupcake can protect databases accessed through MCP (Model Context Protocol) servers. This capability expands to any MCP.

### Setup the Database Demo

⚠️ Requires Docker.

Run the MCP setup script to create a PostgreSQL database with appointment data:

```bash
./mcp_setup.sh # docker must be running for this to work
```

♻️ Reset anytime with:

```bash
./mcp_cleanup.sh
```

This will:

- Start a PostgreSQL Docker container with appointment data
- Install a policy that prevents database deletions and last-minute cancellations
- Configure Cursor to access the database via MCP

### Test Database Protection

After restarting Cursor, try these scenarios:

**Allowed Operations:**

```
> Show me all appointments in the database
```

**Blocked Operations:**

```
> Cancel the appointment for Sarah Johnson
# Blocked - appointment is within 24 hours

> Delete all appointments older than 30 days
# Blocked - no deletions allowed on production data
```

![Cursor MCP protection demo](screenshots/cursor-mcp-demo.png)
_[Screenshot placeholder: Shows MCP database operations being blocked]_

### So How Did That Work?

The appointment cancellation was blocked using **signals** - external scripts that provide runtime data to policies.

## Step 7: Introducing external context for more effective policy evaluation.

Cupcake allows you to configure signals, arbitrary scripts, strings, and commands that can be used in conjunction with the Cursor event. It can take the event as input and use it to query real-world systems that you might need further context from. In the example, there's a Python script that takes the appointment's ID (from the agent tool call parameter) to change the appointment to canceled. That script then queries an external system, the Appointments Database, and calculates whether or not that appointment is within 24 hours. Passes that data back to Cupcake, and Cupcake makes the decision. Ultimately blocking Cursor from executing the action.

```
Cursor                  Cupcake Engine                  Signal Script              Database
     |                         |                               |                        |
     |--beforeMCPExecution---->|                               |                        |
     |  (SQL: UPDATE...id=1)   |                               |                        |
     |                         |--Pipe event JSON via stdin-->|                        |
     |                         |                               |--Query appointment---->|
     |                         |                               |<---Time: 17 hours------|
     |                         |<--{within_24_hours: true}----|                        |
     |                         |                               |                        |
     |                    [Policy evaluates]                   |                        |
     |<---DENY: within 24hrs---|                               |                        |
```

The signal (`check_appointment_time.py`) dynamically extracts the appointment ID from the SQL, queries the database, and returns whether it's within 24 hours. This enables policies to make real-time decisions based on actual data - no hardcoded values.

### When to use signals

1. Use signals anytime you want to enrich an agent event with deeper context and information you can only get at a point in time.

2. Signals also allow you to do advanced guard railing. Cupcake itself does not intend to be a scanning or classifier type of system, such as NVIDIA NeMo or Invariant guardrails. However, you can use those types of guardrails (LLM-based evaluations, AI as a judge, AI classifiers, etc.) to evaluate the tool calls and ultimately make the decision on whether to allow or deny. Cupcake is simple in that it can accept outputs from the advance guardrail systems as the decision. The Cupcake policy is simple in those cases.

### Cleanup

When done testing:

```bash
./mcp_cleanup.sh
```

---

## Key Takeaways

1. **Policies work transparently** - No changes needed to Cursor itself
2. **Built-ins provide baseline security** - Critical paths protected by default
3. **Layered protection** - Global policies + project policies + built-ins
4. **Real-time enforcement** - Commands blocked before execution
5. **AI-resistant** - Agents cannot easily bypass security policies

Explore the policy files in `.cupcake/policies/` to understand how this protection works under the hood.
