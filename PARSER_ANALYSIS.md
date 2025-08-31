# Guidebook Parser Analysis - Security Review

## Critical Finding Confirmed

**Yes, there ARE two separate and incompatible parsers for guidebook.yml**, creating a significant security vulnerability.

## The Two Parsers

### 1. Engine Parser (`cupcake-core/src/engine/guidebook.rs`)
- **Used by**: The main Cupcake engine at runtime
- **Features**:
  - ✅ Convention-based discovery via `load_with_conventions()`
  - ✅ Auto-discovers scripts in `signals/` and `actions/` directories
  - ✅ Supports `on_any_denial` actions
  - ✅ Supports `by_rule_id` action mappings
  - ✅ Complex nested structure with `ActionSection`

**Data Structure:**
```rust
pub struct Guidebook {
    pub signals: HashMap<String, SignalConfig>,
    pub actions: ActionSection,  // Complex nested structure
}

pub struct ActionSection {
    pub on_any_denial: Vec<ActionConfig>,  // Global actions
    pub by_rule_id: HashMap<String, Vec<ActionConfig>>,  // Per-rule actions
}
```

### 2. Trust Parser (`cupcake-core/src/trust/guidebook.rs`)
- **Used by**: The `cupcake trust` CLI commands
- **Features**:
  - ❌ NO convention-based discovery
  - ❌ NO support for `on_any_denial`
  - ❌ Flat action structure (just a HashMap)
  - ❌ Only sees explicitly declared scripts in guidebook.yml

**Data Structure:**
```rust
pub struct Guidebook {
    pub signals: HashMap<String, SignalConfig>,
    pub actions: HashMap<String, ActionConfig>,  // Simple flat map
}
```

## The Security Gap

### What the Engine Sees (Runtime)
1. Scripts explicitly in `guidebook.yml`
2. **+ All scripts auto-discovered from `signals/` directory**
3. **+ All scripts auto-discovered from `actions/` directory**
4. **+ `on_any_denial` actions that run on every denial**

### What the Trust System Sees (Trust Verification)
1. **ONLY** scripts explicitly declared in `guidebook.yml`
2. ❌ Missing convention-discovered scripts
3. ❌ Missing `on_any_denial` actions
4. ❌ Cannot parse the nested action structure

## Proof Points

### Engine Loading (line 275-280 in engine/mod.rs):
```rust
self.guidebook = Some(guidebook::Guidebook::load_with_conventions(
    &self.paths.guidebook,
    &self.paths.signals,    // Auto-discovers scripts here
    &self.paths.actions     // Auto-discovers scripts here
).await?);
```

### Trust CLI Loading (line 111 in trust_cli.rs):
```rust
let guidebook = crate::trust::guidebook::Guidebook::load(project_dir)
    .context("Failed to load guidebook.yml")?;
// No convention discovery, just reads the YAML file
```

## Security Implications

### False Sense of Security
1. User runs `cupcake trust init` - sees "✅ Trust initialized successfully"
2. Trust system only hashes scripts it knows about (from guidebook.yml)
3. Engine at runtime executes additional scripts from directories
4. **These additional scripts are NEVER verified by the trust system**

### Attack Vector
An attacker could:
1. Drop a malicious script in `signals/` or `actions/` directory
2. The trust system won't know about it (not in guidebook.yml)
3. The engine will auto-discover and execute it
4. No trust verification occurs because trust doesn't know it exists

### Example Scenario
```yaml
# guidebook.yml
signals:
  git_branch:
    command: "git branch --show-current"

actions:
  BASH-001:
    - command: "echo 'Violation logged'"
```

Trust system sees: 2 scripts (1 signal, 1 action)

But if these directories exist:
```
signals/
  steal_secrets.sh    # Not in guidebook.yml
actions/  
  exfiltrate.sh       # Not in guidebook.yml
```

Engine executes: 4 scripts (including the malicious ones)
Trust verifies: Only the 2 in guidebook.yml

## Recommended Fix

### Option 1: Unify the Parsers
- Make trust system use the SAME parser as the engine
- Import `cupcake_core::engine::guidebook::Guidebook` in trust CLI
- Ensure trust sees ALL scripts the engine will execute

### Option 2: Remove Convention-Based Discovery
- Require ALL scripts to be explicitly declared in guidebook.yml
- Remove auto-discovery from directories
- Simpler but less convenient

### Option 3: Trust System Discovers Scripts Too
- Update trust parser to also scan directories
- Implement the same discovery logic in both places
- Risk of divergence over time

## Verification Commands

```bash
# See what trust thinks it's protecting
cupcake trust list

# See what actually exists in directories
ls -la .cupcake/signals/
ls -la .cupcake/actions/

# If counts don't match, you have unprotected scripts
```

## Conclusion

This is a **CRITICAL SECURITY VULNERABILITY**. The trust system provides a false sense of security by only protecting a subset of scripts that the engine will actually execute. The two parsers must be unified or the convention-based discovery must be removed entirely.