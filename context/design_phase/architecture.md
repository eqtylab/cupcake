# Cupcake Architecture Document

## Overview

Cupcake is a policy enforcement engine for AI coding agents (starting with Claude Code) that transforms natural language rules in CLAUDE.md files into deterministic, enforceable policies via Claude Code's hook system.

## Core Architecture Principles

1. **Stateless Binary**: Single-purpose executable that runs on each hook event
2. **Performance-First**: Sub-100ms response time via aggressive caching
3. **AI-Powered Translation**: Uses Claude Code itself to translate natural language rules
4. **Stateful from V1**: Maintains session state for complex multi-step policies
5. **Security by Design**: Minimal permissions, transparent operations, no self-modification

## System Components

### 1. The `cupcake` CLI Binary (Rust)

A single statically-linked executable with multiple subcommands:

```
cupcake
├── init      # Interactive policy generation from CLAUDE.md files
├── sync      # Updates Claude Code hooks in settings.json
├── run       # Runtime enforcement (called by hooks)
├── validate  # Validates policy file syntax
└── audit     # Views audit logs (if enabled)
```

### 2. Policy Configuration (`cupcake.toml`)

Human-readable TOML format containing:
- Policy schema version
- Global settings (audit logging, etc.)
- Array of policy definitions
- Each policy includes: name, hook event, matcher, conditions, and actions

### 3. State Management System

- Session-specific state files: `.cupcake/state/<session_id>.json`
- Automatic tracking of all tool usage (Read, Write, Edit, Bash, etc.)
- Append-only event log structure
- Enables complex policies like "must read X before editing Y"
- Custom events via `update_state` action
- Automatically cleaned up after session ends

Example state entry:
```json
{
  "timestamp": "2024-01-10T10:00:00Z",
  "tool": "Read",
  "success": true,
  "input": { "file_path": "docs/architecture.md" }
}
```

### 4. Caching System

- Binary-serialized policy cache: `.cupcake/policy.cache`
- Timestamp-based invalidation
- Near-zero parsing overhead for unchanged policies
- Uses `bincode` for fast serialization

### 5. Audit System

- Optional structured logging to `.cupcake/audit.log`
- JSON Lines format for easy parsing
- Records all policy decisions and outcomes
- Configurable via `cupcake.toml`

## Data Flow

### Initialization Flow (`cupcake init`)

```
1. Discover CLAUDE.md files:
   - Walk up from CWD to root
   - Find all CLAUDE.md in subtrees
   - Resolve @imports (max depth: 5)
   ↓
2. Generate meta-prompt with all content
   ↓
3. Launch Claude Code session
   ↓
4. Claude generates cupcake.toml.tmp
   ↓
5. Validate generated policies
   ↓
6. If invalid: feedback loop to Claude
   ↓
7. User reviews and approves
   ↓
8. Save as cupcake.toml
   ↓
9. Auto-run sync command
```

### Runtime Flow (`cupcake run`)

```
1. Receive hook event JSON via stdin
   ↓
2. Load policies (from cache if possible)
   ↓
3. Build ordered policy list:
   - Project policies (./cupcake.toml) first
   - User policies (~/.claude/cupcake.toml) appended
   ↓
4. PASS 1: Collect all feedback
   - Iterate through entire list
   - Evaluate conditions for each policy
   - Collect ALL "soft" feedback (provide_feedback actions)
   - Don't stop on matches, continue to end
   ↓
5. PASS 2: Check for hard actions
   - Re-iterate through same ordered list
   - Stop at FIRST "hard" action (block/approve)
   - This becomes the final decision
   ↓
6. Combine results:
   - If hard action found: block operation AND include all collected feedback
   - If only feedback: block with all collected feedback
   - If nothing: allow operation
   ↓
7. Update state file if needed
   ↓
8. Log to audit if enabled
   ↓
9. Return unified response to Claude
```

### Two-Pass Aggregation Model

Cupcake uses a two-pass evaluation model that aggregates feedback while respecting critical rules:

**Pass 1: Feedback Collection**
- Evaluates ALL policies in the ordered list
- Collects every piece of feedback from matching policies
- Never stops early - ensures comprehensive feedback

**Pass 2: Hard Action Detection**
- Re-scans for "hard" actions (block_with_feedback, approve, run_command)
- Stops at the FIRST matching hard action
- This becomes the final decision

**Result Combination:**
When both soft feedback and hard blocks are found, ALL feedback is provided:
```
Operation blocked: Tests must pass before committing

Additional policy feedback:
• Use <Button> instead of <button>
• Use <Link> instead of <a>
• Import components from @/ui
```

## Policy Loading Order

Policies are loaded in a specific order that mirrors Claude Code's memory structure:

1. **Project Policies** (`./cupcake.toml`) - Corresponds to `./CLAUDE.md`
2. **User Policies** (`~/.claude/cupcake.toml`) - Corresponds to `~/.claude/CLAUDE.md`

This ordering ensures:
- All feedback is collected from both levels
- Project-level hard blocks take precedence over user preferences
- Natural, intuitive hierarchy matching Claude Code's design

Note: Future versions will support additional settings file locations to match Claude Code's full hierarchy:
- Enterprise managed policy settings
- `.claude/settings.local.json` (local project settings)
- Multiple project/user settings locations

## File Structure

```
project/
├── .claude/
│   └── settings.json          # Claude Code hooks configuration
├── .cupcake/
│   ├── policy.cache          # Binary-serialized policies
│   ├── state/
│   │   └── <session_id>.json # Session state files
│   └── audit.log             # Audit trail (if enabled)
├── cupcake.toml              # Project policies
└── CLAUDE.md                 # Natural language rules (source)

~/.claude/
├── cupcake.toml              # User policies
└── CLAUDE.md                 # User preferences
```

## Integration with Claude Code

### Hook Configuration

Cupcake registers a single entry point per hook type in `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cupcake run --event PreToolUse",
        "timeout": 60
      }]
    }],
    "PostToolUse": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cupcake run --event PostToolUse",
        "timeout": 60
      }]
    }],
    "Notification": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cupcake run --event Notification",
        "timeout": 60
      }]
    }],
    "Stop": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cupcake run --event Stop",
        "timeout": 60
      }]
    }],
    "SubagentStop": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cupcake run --event SubagentStop",
        "timeout": 60
      }]
    }],
    "PreCompact": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cupcake run --event PreCompact",
        "timeout": 60
      }]
    }]
  }
}
```

### Communication Protocol

- **Input**: JSON via stdin (session_id, tool_name, tool_input, etc.)
- **Output**: 
  - Exit code 0: Allow (no output)
  - Exit code 2: Block (stderr → Claude feedback)
  - JSON stdout: Structured decisions

## Performance Optimizations

1. **Policy Caching**: Binary serialization eliminates parsing overhead
2. **Lazy State Loading**: Only read state when policies require it
3. **Compiled Regex**: Pre-compile patterns during policy loading
4. **Memory-Mapped Files**: For large audit logs (future)
5. **Static Linking**: Zero runtime dependencies

## Security Model

1. **Principle of Least Privilege**: 
   - Read-only access to policies
   - Write access only to `.cupcake/` directory
   - Commands run with user permissions

2. **Transparency**:
   - All policies visible in `cupcake.toml`
   - User approval required during init
   - Audit trail available

3. **Input Validation**:
   - Strongly-typed Rust structs for all inputs
   - Path traversal protection
   - Command injection prevention

## Error Handling

1. **Graceful Degradation**: If Cupcake fails, Claude Code continues
2. **Clear Feedback**: Errors returned to Claude for self-correction
3. **Validation Loop**: Invalid policies caught during init
4. **Timeout Protection**: 60-second limit inherited from hooks

## Future Extensibility

The architecture supports future enhancements:

1. **Enterprise Management**: Centralized policy distribution
2. **Policy Templates**: Reusable policy patterns
3. **Advanced State**: Cross-session state persistence
4. **Network Integration**: Remote policy servers
5. **Custom Actions**: Plugin system for actions

## Technology Stack

- **Language**: Rust (performance, safety, single binary)
- **CLI Framework**: `clap` (argument parsing)
- **Serialization**: `serde` + `toml` + `bincode`
- **Regex Engine**: `regex` crate (consistent with ripgrep)
- **JSON Processing**: `serde_json`
- **File Watching**: `notify` (future: live reload)