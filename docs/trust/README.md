# Cupcake Trust System

## Overview

The Cupcake Trust System provides cryptographic integrity verification for scripts executed as signals and actions. This is an **optional** security layer that ensures scripts haven't been modified between approval and execution, preventing potential supply chain attacks where an AI agent might modify trusted scripts.

**Key Principle**: Trust is optional but recommended. Cupcake will inform users about trust mode on first use, but won't require it.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Motivation & Threat Model](#motivation--threat-model)
3. [How It Works](#how-it-works)
4. [Trust Manifest Format](#trust-manifest-format)
5. [Commands Reference](#commands-reference)
6. [Developer Workflows](#developer-workflows)
7. [Security Properties](#security-properties)
8. [Implementation Guide](#implementation-guide)
9. [Future Enhancements](#future-enhancements)

## Quick Start

```bash
# Initialize a Cupcake project
cupcake init

# (Optional) Enable trust mode for script integrity
cupcake trust init
✓ Scanning guidebook.yml for scripts...
✓ Found 5 signals, 3 actions
✓ Trust manifest created: .cupcake/.trust
✓ All scripts fingerprinted and protected

# Normal usage - scripts are now verified before execution
cat event.json | cupcake eval --project-dir .cupcake

# If a script changes, Cupcake will detect it
# ERROR: Script integrity violation!
# Run 'cupcake trust update' to approve changes

# Explicitly approve script changes
cupcake trust update
```

## Motivation & Threat Model

### The Problem

Without trust verification, the following attack is possible:

1. User writes safe scripts for signals/actions
2. User configures policies that use these scripts
3. AI agent (or malicious actor) modifies a script
4. Cupcake executes the modified script with its privileges
5. The modified script performs unintended actions

This creates a Time-of-Check-Time-of-Use (TOCTOU) vulnerability where scripts verified as safe during setup can be modified before execution.

### The Solution

The Trust System creates a cryptographic manifest of all scripts referenced in your guidebook. Before executing any script, Cupcake verifies its integrity against this manifest. Any modifications require explicit user approval.

### Threat Model Assumptions

- **Trusted**: The Cupcake binary and .cupcake/.trust manifest
- **Untrusted**: Any script file that could be modified after initial approval
- **Protected Against**: Unauthorized script modifications, script substitution
- **Not Protected Against**: Cupcake binary replacement, trust manifest deletion

## How It Works

### Trust Modes

Cupcake operates in one of two modes:

1. **Standard Mode** (Default)
   - No integrity checking
   - Scripts execute immediately
   - Suitable for development and low-risk environments
   - Zero friction, maximum flexibility

2. **Trust Mode** (Opt-in)
   - All scripts must be in the trust manifest
   - Cryptographic verification before execution
   - Modifications require explicit approval
   - Suitable for production and high-security environments

### Architecture Integration

```
Cupcake Engine with Trust System:

┌─────────────────────────────────────────────────────────────┐
│                       Cupcake Engine                        │
│  ┌─────────────┐  ┌────────────────┐  ┌─────────────────┐  │
│  │  Policy     │  │    Signal      │  │     Action      │  │
│  │ Evaluation  │  │   Execution    │  │   Execution     │  │
│  │             │  │                │  │                 │  │
│  └─────────────┘  └────────┬───────┘  └─────────┬───────┘  │
│                           │                     │          │
└───────────────────────────┼─────────────────────┼──────────┘
                            │                     │
                    ┌───────▼─────────┐   ┌──────▼──────┐
                    │ Trust Verifier  │   │ Trust       │
                    │ (Optional)      │   │ Verifier    │
                    │                 │   │ (Optional)  │
                    │ • Hash Check    │   │             │
                    │ • Manifest      │   │ • Hash Check│
                    │   Lookup        │   │ • HMAC      │
                    └─────────────────┘   └─────────────┘
                            │                     │
                    ┌───────▼─────────┐   ┌──────▼──────┐
                    │  .cupcake/      │   │ Security    │
                    │    .trust       │   │ Violation   │
                    │                 │   │ Handling    │
                    │ • Script Hashes │   │             │
                    │ • HMAC Signature│   │ • Block     │
                    │ • Metadata      │   │ • Alert     │
                    └─────────────────┘   └─────────────┘
```

### Startup Notification

When Cupcake starts without trust mode:

```
┌─────────────────────────────────────────────────────────┐
│ Cupcake is running in STANDARD mode                      │
│                                                           │
│ Script integrity verification is DISABLED.               │
│ Enable trust mode for enhanced security:                 │
│   $ cupcake trust init                                   │
│                                                           │
│ Learn more: cupcake trust --help                         │
└─────────────────────────────────────────────────────────┘
```

### Core Workflow

1. **Initialization**: `cupcake trust init` scans your guidebook.yml and creates a manifest
2. **Verification**: Before each script execution, Cupcake verifies its hash
3. **Violation Detection**: Modified scripts are blocked from execution
4. **Explicit Updates**: `cupcake trust update` allows you to approve changes

### Runtime Execution Flow

```
User Script Execution (Signal/Action):

┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Script Call   │───▶│ Trust Enabled?   │───▶│   Execute       │
│   (Signal/      │    │                  │    │   Script        │
│    Action)      │    │ Standard Mode    │    │   Immediately   │
└─────────────────┘    │ → Skip Check     │    └─────────────────┘
                       └──────────────────┘
                                │
                          Trust Mode ▼
                       ┌──────────────────┐
                       │ Load .trust      │
                       │ Manifest         │
                       └──────────────────┘
                                │
                                ▼
                       ┌──────────────────┐
                       │ Compute Current  │
                       │ Script Hash      │
                       │ (SHA-256)        │
                       └──────────────────┘
                                │
                                ▼
                       ┌──────────────────┐
                       │ Find Script in   │
                       │ Manifest         │
                       └──────────────────┘
                                │
                      ┌─────────▼─────────┐
                      │                   │
                      ▼                   ▼
               ┌─────────────┐     ┌─────────────┐
               │ Hash Match  │     │ Hash Mismatch│
               │             │     │             │
               │ ✅ Execute  │     │ ❌ Block &  │
               │   Script    │     │   Error     │
               └─────────────┘     └─────────────┘
                                          │
                                          ▼
                                  ┌─────────────┐
                                  │ Show Error: │
                                  │ "Run trust  │
                                  │  update"    │
                                  └─────────────┘
```

### Trust Lifecycle

```
Project Setup:
cupcake init → cupcake trust init → Scripts Approved & Hashed

Development Cycle:
Script Modified → Execution Blocked → cupcake trust update → Approval → Continue

Security Event:
Script Tampered → HMAC Verification Failed → Security Alert → Manual Investigation
```

## Trust Manifest Format

The trust manifest (`.cupcake/.trust`) is a JSON file containing cryptographic hashes of all approved scripts.

### Quick Reference

| Script Type | Example Command | Hash Source |
|-------------|-----------------|-------------|
| **Inline** | `npm test` | Command string itself |
| **File** | `./scripts/lint.sh` | Script file contents |
| **Complex** | `python analyzer.py --args` | Script file + arguments |

### Complete Manifest Structure

```json
{
  "version": 1,
  "timestamp": "2024-01-20T10:00:00Z",
  "mode": "enabled",
  "scripts": {
    "signals": {
      "test_status": {
        "type": "inline",
        "command": "npm test",
        "hash": "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
      },
      "lint_check": {
        "type": "file",
        "path": "./scripts/lint.sh",
        "absolute_path": "/Users/john/project/scripts/lint.sh",
        "hash": "sha256:def456abc789...",
        "size": 1024,
        "modified": "2024-01-19T09:30:00Z"
      },
      "code_analysis": {
        "type": "complex",
        "command": "python ~/tools/analyzer.py --mode=quick",
        "components": {
          "interpreter": "python",
          "script": {
            "path": "~/tools/analyzer.py",
            "resolved": "/Users/john/tools/analyzer.py",
            "hash": "sha256:xyz789..."
          },
          "args": ["--mode=quick"]
        }
      }
    },
    "actions": {
      "on_deny_BASH_001": {
        "type": "file",
        "path": ".cupcake/actions/alert.sh",
        "absolute_path": "/Users/john/project/.cupcake/actions/alert.sh",
        "hash": "sha256:qrs456...",
        "size": 256,
        "modified": "2024-01-18T14:20:00Z"
      }
    }
  }
}

/* HMAC: hmac-sha256:1a2b3c4d5e6f... */
```

### Manifest Fields

- **version**: Manifest format version for future compatibility
- **timestamp**: When the manifest was created/updated
- **mode**: "enabled" or "disabled" for trust state
- **scripts**: Nested structure matching guidebook organization
- **HMAC comment**: HMAC signature using derived key for tamper detection

### Script Entry Types

1. **Inline Commands** (`type: "inline"`)
   - Simple commands like `npm test`, `cargo build`
   - Hash computed from the command string itself

2. **File Scripts** (`type: "file"`)
   - Direct script execution: `./lint.sh`, `/usr/local/bin/check.py`
   - Hash computed from file contents
   - Stores absolute path for verification

3. **Complex Commands** (`type: "complex"`)
   - Interpreter + script: `python analyzer.py`, `node build.js`
   - Separately hashes the script file
   - Preserves arguments for complete verification

## Commands Reference

### `cupcake trust init`

Initialize trust mode for the current project.

```bash
cupcake trust init [OPTIONS]

Options:
  --project-dir <PATH>  Path to .cupcake directory [default: .]
  --empty               Skip scanning for existing scripts
```

**Behavior**:
- Scans guidebook.yml for all script references (unless --empty)
- Computes SHA-256 hashes for each script
- Creates .cupcake/.trust manifest
- Enables trust mode for future executions

### `cupcake trust update`

Update the trust manifest after script modifications.

```bash
cupcake trust update [OPTIONS]

Options:
  --project-dir <PATH>  Path to .cupcake directory [default: .cupcake]
  --yes                 Automatically approve all changes
  --dry-run            Show changes without updating
```

**Behavior**:
- Compares current scripts against trust manifest
- Shows all changes (modified, added, removed)
- Prompts for confirmation (unless --yes)
- Updates manifest with new hashes

**Example Output**:
```
Detected changes:
  ~ ./scripts/lint.sh (modified)
    Old hash: sha256:abc123...
    New hash: sha256:def456...
  + ./scripts/new-check.sh (added)
  - ./scripts/old-test.sh (removed)

Update trust manifest? [y/N]: y
✓ Trust manifest updated
```

### `cupcake trust verify`

Verify current scripts against the trust manifest without updating.

```bash
cupcake trust verify [OPTIONS]

Options:
  --project-dir <PATH>  Path to .cupcake directory [default: .cupcake]
  --verbose             Show verification details for each script
```

**Exit Codes**:
- 0: All scripts match manifest
- 1: One or more scripts modified

### `cupcake trust list`

Display all currently trusted scripts.

```bash
cupcake trust list [OPTIONS]

Options:
  --project-dir <PATH>  Path to .cupcake directory [default: .cupcake]
  --modified           Show only modified scripts
  --hashes             Show script hashes
```

**Example Output**:
```
Trusted Scripts (Trust Mode: ENABLED)
Last Updated: 2024-01-20 10:00:00

SIGNALS (3):
  test_status    inline   npm test                    sha256:9f86d0...
  lint_check     file     ./scripts/lint.sh           sha256:def456...
  code_analysis  complex  python ~/tools/analyzer.py  sha256:xyz789...

ACTIONS (2):
  on_deny        file     .cupcake/actions/alert.sh   sha256:qrs456...
  on_any_denial  inline   echo "Denied" >> audit.log  sha256:tuv789...
```

### `cupcake trust disable`

Temporarily disable trust verification (converts to standard mode).

```bash
cupcake trust disable [OPTIONS]

Options:
  --project-dir <PATH>  Path to .cupcake directory [default: .cupcake]

Warning: This disables script integrity verification!
Continue? [y/N]: 
```

### `cupcake trust enable`

Re-enable trust verification (requires existing manifest).

```bash
cupcake trust enable [OPTIONS]

Options:
  --project-dir <PATH>  Path to .cupcake directory [default: .cupcake]
  --verify              Verify all scripts before enabling
```

### `cupcake trust reset`

Remove trust manifest and disable trust mode.

```bash
cupcake trust reset [OPTIONS]

Options:
  --project-dir <PATH>  Path to .cupcake directory [default: .cupcake]
  --force               Skip confirmation prompt

Warning: This will delete the trust manifest!
Continue? [y/N]: 
```

## Developer Workflows

### Initial Project Setup

```bash
# 1. Create a new Cupcake project
cupcake init
cd my-project

# 2. Write your policies
vim .cupcake/policies/bash_safety.rego

# 3. Configure signals and actions in guidebook
vim .cupcake/guidebook.yml

# 4. (Optional but recommended) Enable trust mode
cupcake trust init
# ✓ Trust mode enabled with 8 scripts protected

# 5. Test your setup
cat test-event.json | cupcake eval --project-dir .cupcake
```

### Development Iteration

```bash
# Modify a signal script
vim ./scripts/validation.sh

# Try to run Cupcake - it detects the change
cat event.json | cupcake eval --project-dir .cupcake
# ERROR: Script integrity violation!
# ./scripts/validation.sh has been modified
# Expected: sha256:abc123...
# Actual:   sha256:xyz789...
# Run 'cupcake trust update' to approve changes

# Review and approve the change
cupcake trust update
# Shows diff, prompts for confirmation

# Continue development
cat event.json | cupcake eval --project-dir .cupcake
# ✓ Works normally
```

### CI/CD Integration

```bash
# In CI pipeline - fail on trust violations
cupcake trust verify || exit 1

# Auto-update trust in development branches
if [ "$BRANCH" = "development" ]; then
  cupcake trust update --yes
fi

# Strict verification in production
if [ "$ENV" = "production" ]; then
  cupcake trust verify --verbose
  if [ $? -ne 0 ]; then
    echo "ERROR: Trust verification failed in production!"
    exit 1
  fi
fi
```

### Team Collaboration

```bash
# Developer A makes changes
vim ./scripts/new-check.sh
cupcake trust update
git add .cupcake/.trust ./scripts/new-check.sh
git commit -m "Add new validation script"
git push

# Developer B pulls changes
git pull
cupcake trust verify
# ✓ All scripts verified successfully

# Trust manifest is part of version control
# Team always has consistent script integrity
```

## Security Properties

### Security Summary

**✅ Trust Mode Protects Against:**
- TOCTOU (Time-of-Check-Time-of-Use) attacks
- Unauthorized script modifications by AI agents
- Script file tampering between approval and execution
- Supply chain attacks via script substitution

**❌ Trust Mode Does NOT Protect Against:**
- Cupcake binary replacement or tampering
- System-level compromises (root access)
- Social engineering attacks on users

### What Trust Mode Provides

1. **Integrity Verification**: Detects any modification to trusted scripts
2. **Tamper Evidence**: HMAC signature prevents manifest tampering
3. **Explicit Approval**: All script changes require user confirmation
4. **Complete Coverage**: Verifies both file scripts and inline commands
5. **Path Traversal Protection**: Resolves and verifies absolute paths

### What Trust Mode Does NOT Provide

1. **Confidentiality**: Scripts remain readable (not encrypted)
2. **Authentication**: No verification of script authorship
3. **Runtime Sandboxing**: Scripts still run with Cupcake's privileges
4. **Binary Protection**: Cupcake binary itself is not verified

### Key Derivation

The HMAC key is derived from multiple entropy sources:

```
Key = SHA256(
  "CUPCAKE_TRUST_V1" ||
  cupcake_binary_path ||
  machine_id ||
  username ||
  project_directory
)
```

This provides:
- **Uniqueness**: Different key per machine/user/project
- **Reproducibility**: Same key generated consistently
- **No External Dependencies**: No key management required

## Implementation Guide

### Core Components

#### 1. Trust Manifest Manager (`src/trust/manifest.rs`)

```rust
pub struct TrustManifest {
    version: u32,
    timestamp: DateTime<Utc>,
    mode: TrustMode,
    policy_hash: String,
    scripts: ScriptManifest,
    hmac: String,
}

impl TrustManifest {
    pub fn init(guidebook: &Guidebook) -> Result<Self>
    pub fn verify_script(&self, reference: &ScriptRef) -> Result<()>
    pub fn update(&mut self, guidebook: &Guidebook) -> Result<ChangeSet>
    pub fn compute_hmac(&self) -> Result<String>
    pub fn verify_hmac(&self) -> Result<bool>
}
```

#### 2. Script Hasher (`src/trust/hasher.rs`)

```rust
pub struct ScriptHasher;

impl ScriptHasher {
    pub fn hash_file(path: &Path) -> Result<String>
    pub fn hash_command(command: &str) -> Result<String>
    pub fn hash_complex_command(cmd: &ComplexCommand) -> Result<ScriptHash>
}
```

#### 3. Trust Verifier (`src/trust/verifier.rs`)

```rust
pub struct TrustVerifier {
    manifest: TrustManifest,
    strict: bool,
}

impl TrustVerifier {
    pub fn verify_before_execution(&self, script: &ScriptRef) -> Result<()>
    pub fn verify_all(&self, guidebook: &Guidebook) -> Result<VerificationReport>
}
```

#### 4. Engine Integration (`src/engine/mod.rs`)

```rust
impl Engine {
    async fn execute_signal(&self, signal: &Signal) -> Result<Value> {
        // Check if trust mode is enabled
        if let Some(trust) = &self.trust_verifier {
            trust.verify_before_execution(&signal.script_ref)?;
        }
        
        // Proceed with execution
        self.execute_script_internal(&signal.command).await
    }
}
```

### Error Handling

Trust violations should be clear and actionable:

```rust
pub enum TrustError {
    ManifestNotFound,
    ManifestTampered,
    ScriptModified { 
        path: PathBuf, 
        expected: String, 
        actual: String 
    },
    ScriptNotTrusted { 
        path: PathBuf 
    },
    ScriptNotFound { 
        path: PathBuf 
    },
}

impl Display for TrustError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            TrustError::ScriptModified { path, .. } => {
                write!(f, 
                    "Script integrity violation: {}\n\
                     Run 'cupcake trust update' to approve changes.",
                    path.display())
            }
            // ... other cases
        }
    }
}
```

### Performance Considerations

1. **Cache Hashes**: Store computed hashes in memory during execution
2. **Lazy Loading**: Only verify scripts that are actually executed
3. **Parallel Hashing**: Use rayon for initial manifest creation
4. **Incremental Updates**: Only re-hash modified files

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_trust_init_creates_manifest() { }
    
    #[test]
    fn test_modified_script_detection() { }
    
    #[test]
    fn test_hmac_tamper_detection() { }
    
    #[test]
    fn test_complex_command_parsing() { }
    
    #[test]
    fn test_trust_mode_optional() { }
}
```

## Future Enhancements

### Enterprise Features (Future)

1. **Hardware Key Support**
   - YubiKey / TPM integration
   - Hardware-backed HMAC signing

2. **Centralized Trust Registry**
   - Team-wide trust management
   - Policy distribution system

3. **Audit Logging**
   - Cryptographic audit trail
   - Compliance reporting

4. **Multi-Signature Requirements**
   - Require N-of-M approvals for trust updates
   - Role-based trust management

5. **External Attestation**
   - Integration with signing services
   - Third-party verification

### Platform-Specific Enhancements

1. **macOS**: Keychain integration for key storage
2. **Windows**: Windows Credential Manager integration  
3. **Linux**: Linux kernel keyring integration

### Advanced Verification

1. **Static Analysis**: Analyze script contents for dangerous patterns
2. **Behavioral Analysis**: Monitor script execution patterns
3. **Allowlisting**: Restrict to specific interpreters/commands

## Summary

The Cupcake Trust System provides optional cryptographic integrity verification for scripts without changing how developers write or organize their code. It's designed to be:

- **Optional**: Users choose when to enable it
- **Zero-friction**: Scripts stay wherever they are
- **Explicit**: Changes require deliberate approval
- **Simple**: No complex key management
- **Secure**: Cryptographically sound protection

The trust system follows Cupcake's philosophy: simple for users, intelligent in implementation. By making trust optional but easy to enable, we provide security for those who need it without forcing complexity on those who don't.