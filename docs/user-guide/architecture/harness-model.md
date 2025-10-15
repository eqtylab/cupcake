# Harness-Specific Architecture

Cupcake uses a **harness-specific architecture** that provides first-class support for multiple AI coding agents (Claude Code, Cursor, etc.) without compromising on simplicity or power.

## Overview

A **harness** in Cupcake refers to the AI coding agent platform that invokes policy evaluation. Each harness has:

- **Unique event formats** - Different JSON structures for hook events
- **Unique response formats** - Different expected responses for allow/deny decisions
- **Different capabilities** - Varying support for context injection, file access, etc.

Rather than abstracting away these differences, Cupcake embraces them through a **harness-specific model**.

---

## Core Principles

### 1. Explicitness Over Magic

The harness is **always explicit**, never inferred:

```bash
# ✅ Explicit harness specification
cupcake eval --harness cursor < event.json

# ❌ No auto-detection (prevents ambiguity)
cupcake eval < event.json  # ERROR: --harness flag required
```

**Why?** Explicit harness selection:
- Eliminates debugging confusion ("which event format is this?")
- Prevents subtle bugs from mis-detection
- Makes the data flow transparent
- Enables better error messages

### 2. Native Event Formats

Events flow through Cupcake in their **native format** - no translation or normalization:

```
┌─────────┐   Native JSON   ┌──────────┐   Native JSON   ┌──────────┐
│  Cursor │ ─────────────▶  │ Cupcake  │ ─────────────▶  │  Policy  │
└─────────┘                  │  Engine  │                  │  (Rego)  │
                             └──────────┘                  └──────────┘
                                   │
                                   ▼
                            Cursor Response
```

**Example: Cursor Event (Unchanged)**
```json
{
  "hook_event_name": "beforeShellExecution",
  "conversation_id": "conv_123",
  "command": "git push"
}
```

This native event is passed directly to the policy. No `tool_name` translation. No `tool_input` wrapping.

**Benefits:**
- Policies can access **100% of event data** (no information loss)
- Debugging is straightforward (input JSON == what policy sees)
- New harnesses are easier to add (no normalization layer to maintain)

### 3. Harness-Specific Policy Directories

Policies are **physically separated** by harness:

```
.cupcake/
└── policies/
    ├── claude/          # Claude Code policies
    │   ├── system/
    │   │   └── evaluate.rego
    │   └── my_policy.rego
    └── cursor/          # Cursor policies
        ├── system/
        │   └── evaluate.rego
        └── my_policy.rego
```

The engine loads **only** the policies for the specified harness:

```bash
# Loads policies from .cupcake/policies/cursor/ ONLY
cupcake eval --harness cursor
```

**Benefits:**
- No policy conflicts between harnesses
- Clear organization
- Easy to see which rules apply to which agent
- Policies can use harness-specific event fields without compromise

---

## Architecture Components

### Component 1: CLI Layer (Harness Entry Point)

**File:** `cupcake-cli/src/main.rs`

The CLI accepts the `--harness` flag and passes it to the engine:

```rust
#[derive(Parser)]
enum Command {
    Eval {
        #[clap(long, value_enum)]
        harness: HarnessType,  // REQUIRED
        // ...
    }
}
```

The `HarnessType` enum defines supported harnesses:

```rust
#[derive(Debug, Clone, ValueEnum)]
enum HarnessType {
    Claude,   // Claude Code (claude.ai/code)
    Cursor,   // Cursor (cursor.sh)
}
```

### Component 2: Event Structures

**Files:** `cupcake-core/src/harness/events/{claude_code,cursor}/`

Each harness has dedicated event structures matching its native JSON format:

**Cursor Example:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name")]
pub enum CursorEvent {
    #[serde(rename = "beforeShellExecution")]
    BeforeShellExecution(BeforeShellExecutionPayload),

    #[serde(rename = "beforeReadFile")]
    BeforeReadFile(BeforeReadFilePayload),
    // ...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeforeShellExecutionPayload {
    pub conversation_id: String,
    pub generation_id: String,
    pub workspace_roots: Vec<String>,
    pub command: String,  // Native field name!
}
```

**Claude Code Example:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name")]
pub enum ClaudeCodeEvent {
    PreToolUse {
        tool_name: String,
        tool_input: Value,  // Different structure!
        // ...
    },
    // ...
}
```

### Component 3: Engine (Harness-Aware Evaluation)

**File:** `cupcake-core/src/engine/mod.rs`

The engine accepts a harness parameter and uses it to:

1. **Load the correct policy directory:**

```rust
let harness_dir = match harness {
    HarnessType::Claude => "claude",
    HarnessType::Cursor => "cursor",
};

let policy_path = config_paths.policies.join(harness_dir);
```

2. **Pass native events to policies:**

```rust
// No normalization - event goes through as-is
let decision = engine.evaluate(&native_event_json, None).await?;
```

3. **Return generic decisions:**

```rust
pub enum FinalDecision {
    Allow { context: Vec<String> },
    Deny { reason: String, severity: String, rule_id: String },
    Halt { reason: String, severity: String, rule_id: String },
    Ask { reason: String, question: String, severity: String, rule_id: String },
}
```

The engine output is **harness-agnostic** (standard decision types).

### Component 4: Response Builders

**Files:** `cupcake-core/src/harness/response/{claude_code,cursor}/`

Each harness has response builders that translate generic decisions into harness-specific JSON:

**Cursor Response Builder:**
```rust
impl CursorResponseBuilder {
    pub fn build_response(
        event: &CursorEvent,
        decision: &FinalDecision,
    ) -> Result<Value> {
        match event {
            CursorEvent::BeforeShellExecution(_) => {
                // Cursor-specific format
                match decision {
                    FinalDecision::Deny { reason, .. } => json!({
                        "permission": "deny",
                        "userMessage": reason,
                        "agentMessage": reason,
                    }),
                    // ...
                }
            }
        }
    }
}
```

**Claude Code Response Builder:**
```rust
impl ClaudeHarness {
    pub fn format_response(
        event: &ClaudeCodeEvent,
        decision: &FinalDecision,
    ) -> Result<Value> {
        // Claude-specific format (different from Cursor!)
        match decision {
            FinalDecision::Deny { reason, .. } => json!({
                "continue": false,
                "stopReason": reason,
            }),
            // ...
        }
    }
}
```

---

## Data Flow

Here's the complete flow for a Cursor event:

```
1. Cursor sends hook event
   │
   ▼
   {
     "hook_event_name": "beforeShellExecution",
     "command": "rm -rf /"
   }

2. CLI receives event + --harness cursor flag
   │
   ▼
   cupcake eval --harness cursor < event.json

3. Engine loads policies from cursor/ directory
   │
   ▼
   .cupcake/policies/cursor/*.rego

4. Policy evaluates NATIVE event
   │
   ▼
   deny contains decision if {
       input.hook_event_name == "beforeShellExecution"
       contains(input.command, "rm -rf")  # Native field!
       decision := { "rule_id": "...", "reason": "..." }
   }

5. Engine returns generic decision
   │
   ▼
   FinalDecision::Deny {
       reason: "Dangerous command",
       severity: "CRITICAL",
       rule_id: "CURSOR-001"
   }

6. Response builder formats for Cursor
   │
   ▼
   {
     "permission": "deny",
     "userMessage": "Dangerous command",
     "agentMessage": "Policy CURSOR-001 blocked"
   }

7. Cursor receives response and blocks action
```

---

## Policy Authoring Model

### Harness-Specific Policies

Policies are written for a **specific harness**, accessing native event fields:

**Cursor Policy:** `.cupcake/policies/cursor/block_rm.rego`
```rego
package cursor.policies.block_rm

deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "rm -rf")  # Cursor's native field
    decision := {...}
}
```

**Claude Policy:** `.cupcake/policies/claude/block_rm.rego`
```rego
package claude.policies.block_rm

deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf")  # Claude's native field
    decision := {...}
}
```

### Shared Logic Pattern

To reduce duplication, extract common logic into shared modules:

**Common Module:** `.cupcake/policies/common/utils.rego`
```rego
package common.utils

is_dangerous_rm_command(cmd) {
    contains(lower(cmd), "rm")
    contains(cmd, "-rf")
    contains(cmd, "/")
}
```

**Cursor Policy (Using Shared):**
```rego
package cursor.policies.block_rm
import data.common.utils.is_dangerous_rm_command

deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    is_dangerous_rm_command(input.command)  # Shared logic
    decision := {...}
}
```

**Claude Policy (Using Shared):**
```rego
package claude.policies.block_rm
import data.common.utils.is_dangerous_rm_command

deny contains decision if {
    input.tool_name == "Bash"
    is_dangerous_rm_command(input.tool_input.command)  # Shared logic
    decision := {...}
}
```

This pattern:
- Avoids duplication of business logic
- Maintains harness-specific field access
- Makes policies easier to maintain

---

## Built-in Policies

Built-in policies follow the same harness-specific model:

```
fixtures/
├── claude/
│   └── builtins/
│       ├── git_block_no_verify.rego      # Claude version
│       └── protected_paths.rego
└── cursor/
    └── builtins/
        ├── git_block_no_verify.rego      # Cursor version
        └── protected_paths.rego
```

Each harness has its own implementation accessing native event fields.

---

## Global Configuration

Global policies also use harness-specific directories:

```
~/.config/cupcake/           # or %APPDATA%\cupcake on Windows
└── policies/
    ├── claude/
    │   ├── system/
    │   │   └── evaluate.rego
    │   └── company_policies.rego
    └── cursor/
        ├── system/
        │   └── evaluate.rego
        └── company_policies.rego
```

Global policies are evaluated **first** and can enforce organization-wide rules that cannot be overridden by project policies.

---

## Adding New Harnesses

The architecture makes adding new harnesses straightforward:

1. **Add to `HarnessType` enum** (CLI)
2. **Create event structures** matching the harness's native format
3. **Create response builders** for the harness's expected response format
4. **Add policy directory** (`policies/newharnress/`)
5. **Implement `HarnessConfig` trait** for `cupcake init` support

No changes needed to:
- The engine core
- Existing harness implementations
- The routing system
- The decision synthesis

---

## Benefits of This Architecture

### For Policy Authors

- **Full event access** - No hidden fields or transformations
- **Clear organization** - Policies are grouped by harness
- **Type safety** - Policies match harness event schemas
- **Debuggability** - Input JSON matches policy `input.*` exactly

### For Cupcake Developers

- **Separation of concerns** - Each harness is independent
- **No translation bugs** - No normalization layer to maintain
- **Easy extension** - New harnesses don't affect existing ones
- **Simple testing** - Each harness can be tested in isolation

### For End Users

- **Predictability** - Explicit harness selection eliminates surprises
- **Performance** - No normalization overhead
- **Flexibility** - Can use harness-specific features without compromise

---

## Design Tradeoffs

### Tradeoff: Code Duplication vs. Explicitness

**Cost:** Policies must be written separately for each harness (though shared modules reduce duplication).

**Benefit:** Policies are simple, explicit, and have access to all event data.

**Decision:** We chose explicitness. The shared module pattern (`.cupcake/policies/common/`) provides enough code reuse for business logic.

### Tradeoff: Flexibility vs. Portability

**Cost:** Policies aren't automatically portable between harnesses.

**Benefit:** Policies can leverage each harness's unique capabilities.

**Decision:** We chose flexibility. Users can choose to write cross-harness policies using shared modules, or harness-specific policies when needed.

---

## Comparison with Normalization Approach

An alternative architecture would normalize all events to a common format. Here's why we didn't:

| Aspect | Normalization | Harness-Specific (Cupcake) |
|--------|---------------|----------------------------|
| Event access | Limited to normalized fields | Full native event access |
| Debugging | "What got normalized to what?" | Input JSON == policy input |
| Performance | Translation overhead | Direct pass-through |
| Maintainability | Normalization layer to maintain | Independent harness modules |
| Feature parity | Limited by common denominator | Each harness's full feature set |
| Policy portability | High (same policy for all) | Medium (shared modules pattern) |

The harness-specific model trades some portability for **simplicity, power, and debuggability**.

---

## Further Reading

- [Cursor Integration Guide](../harnesses/cursor.md) - Using Cupcake with Cursor
- [Claude Code Integration Guide](../harnesses/claude-code.md) - Using Cupcake with Claude Code
- [Writing Policies](../policies/writing-policies.md) - Policy authoring guide
- [Harness Comparison Matrix](../harnesses/harness-comparison.md) - Feature comparison

---

## Technical Deep Dive

For implementation details, see:
- [Developer: Policy Routing System](../../developer/policy-routing-system.md)
- [Source: Event Structures](https://github.com/eqtylab/cupcake/tree/main/cupcake-core/src/harness/events)
- [Source: Response Builders](https://github.com/eqtylab/cupcake/tree/main/cupcake-core/src/harness/response)
