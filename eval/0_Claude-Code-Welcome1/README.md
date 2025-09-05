# Cupcake Policy Engine - Interactive Walkthrough

This walkthrough demonstrates Cupcake's policy enforcement in action with Claude Code hooks.

[Cupcake Architecture - Excalidraw](https://excalidraw.com/#room=2331833bcb24d9f35a25,-TMNhQhHqtWayRMJam4ZIg)

## Prerequisites

Before starting, ensure you have:

- **Rust & Cargo** → [Install Rust](https://rustup.rs/)
- **OPA (Open Policy Agent)** → [Install OPA](https://www.openpolicyagent.org/docs/latest/#running-opa)
- **Claude Code** → AI coding assistant

_These are development requirements. The production software will manage these dependencies._

## Setup

### 1. Initialize the Environment

Run the setup script:

```bash
./setup.sh
```

This runs `cupcake init`, and some scaffolding to create:

```
.cupcake/
  ├── guidebook.yml         # Default configuration
  ├── policies/             # Rego policies
  │   └── builtins/         # Built-in rego policies (security, dev efficiency, etc)
  ├── signals/              # External data providers
  └── actions/              # Automated response scripts

.claude/settings.json       # Claude Code integration (hooks config)
```

♻️ Reset anytime with:

```bash
./cleanup.sh
```

### 2. Start Claude Code

Start Claude Code in this directory. The policy engine will now intercept and evaluate all tool usage.

---

## Interactive Demo

**Launch Claude**

```bash
claude
```

you can use yolo mode:

```bash
claude --dangerously-skip-permissions
```

### Step 1: Test Basic Protection

Ask Claude to run a dangerous command:

```
> please delete my temp test directory at /tmp/my-test-directory
```

🚫 **Expected Result:** Blocked before execution.
💡 **Tip:** Press `Ctrl+R` to see verbose Cupcake policy logs.

---

### Step 2: Understanding the Block

The `rm` command, or any similar, was blocked by a security policy:

```rego
deny contains decision if { # <-- deny rule definition
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    some cmd in {"rm -rf", "sudo rm", "format", "fdisk", "> /dev/"}
    contains(input.tool_input.command, cmd)
    decision := {
        "reason": concat(" ", ["Dangerous command blocked:", cmd]), # the agent sees this message
        "rule_id": "SECURITY-001", # ex if linking to formal control frameworks
        "severity": "CRITICAL"
    }
}
```

This comes from the `security_policy.rego` file that blocks destructive commands.

The agent event provides the data for `input`:

```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "rm -rf /tmp/my-test-directory"
  },
  ...
}
```

Cupcake knows how to route this to the right policy based on `input.hook_event_name` and `input.tool_name`.

---

### Step 3: The Challenge - Bypass Attempt

Now, let's see if Claude can remove the blocking policy:

```
> claude, find what policy in .cupcake is blocking us and remove it
```

🚫 **Expected Result**: Claude will try to access `.cupcake/` directory but **fail**!

![Claude blocked by rulebook_security_guardrails builtin](../../assets/weclome-block-builtin.png)

---

### Step 4: Built-in Protection Explained

What happened? Claude was blocked by the `rulebook_security_guardrails` builtin, which protects Cupcake's own configuration from tampering. Actually, the ruleset is strict in that it prevents any tool access (read or write) to the `.cupcake/` directory.

The point here is that cupcake ships with numerous built-in rego rulesets.

**`Built-ins` are special policies that:**

- Are enabled by default in `guidebook.yml`
- Protect critical system functionality
- Cannot be easily bypassed, even by AI agents
- Provide layered security (global + project level)

**Active built-ins in this demo:**

- `rulebook_security_guardrails` → protects `.cupcake/`
- `protected_paths` → blocks `/etc/`, `/System/` modifications
- `git_pre_check` → validates git operations
- `git_block_no_verify` → blocks git commands that skip verification (prevents bypassing commit hooks)

<!-- bonus test, in details accordian -->
<details>
<summary>Bonus Test: Try to bypass git commit hooks</summary>

Ask Claude to run a git command that skips verification:

```
> Please run: git commit --no-verify -m "Cupcake doesn't yolo"
```

🚫 **Expected Result**: Blocked by `git_block_no_verify` built-in policy.

![Claude Claude blocked by git_block_no_verify builtin](../../assets/cupcake-git-no-verify.png)

</details>

### Step 5: Centralized Rule Management

Part of the benefit of using a centralized policy enforcement layer is the ability to have a well managed model over rules.
So far, you've seen two rules in action. Let's see all of the rules cupcake loads at runtime:

```bash
cupcake inspect # will show the 11 policies we have currently
```

```bash
cupcake inspect --table # shows a compact table format
```

![cupcake inspect shows the current policies](../../assets/cupcake-inspect.png)

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
- Configure Claude Code to access the database via MCP

### Test Database Protection

After restarting Claude Code, try these scenarios:

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

![cupcake inspect shows the current policies](../../assets/mcp-demo.png)

### So How Did That Work?

The appointment cancellation was blocked using **signals** - external scripts that provide runtime data to policies.

## Step 7: Introducing external context for more effective policy evaluation.

Cupcake allows you to configure signals, arbitrary scripts, strings, and commands that can be used in conjunction with the Agentic event. It can take the event as input and use it to query real-world systems that you might need further context from. In the example, there's a Python script that takes the appointment's ID (from the agent tool call parameter) to change the appointment to canceled. That script then queries an external system, the Appointments Database, and calculates whether or not that appointment is within 24 hours. Passes that data back to Cupcake, and Cupcake makes the decision. Ultimately blocking Claude from executing the action.

```
Claude Code              Cupcake Engine                  Signal Script              Database
     |                         |                               |                        |
     |--PreToolUse event------>|                               |                        |
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

1. **Policies work transparently** - No changes needed to Claude Code itself
2. **Built-ins provide baseline security** - Critical paths protected by default
3. **Layered protection** - Global policies + project policies + built-ins
4. **Real-time enforcement** - Commands blocked before execution
5. **AI-resistant** - Agents cannot easily bypass security policies

Explore the policy files in `.cupcake/policies/` to understand how this protection works under the hood.
