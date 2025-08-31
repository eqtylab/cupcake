# Testing the Trust System Fix

## Setup Test Environment

Create a test project with both explicit and auto-discovered scripts:

```bash
# Create test structure
mkdir -p test-trust/.cupcake/{signals,actions}

# Create guidebook.yml with one explicit signal
cat > test-trust/.cupcake/guidebook.yml << 'EOF'
signals:
  explicit_signal:
    command: "echo 'I am explicit'"

actions:
  on_any_denial:
    - command: "echo 'Global action'"
  by_rule_id:
    RULE-001:
      - command: "echo 'Rule specific'"
EOF

# Create auto-discovered scripts (NOT in guidebook.yml)
echo '#!/bin/bash
echo "I am auto-discovered signal"' > test-trust/.cupcake/signals/auto_signal.sh
chmod +x test-trust/.cupcake/signals/auto_signal.sh

echo '#!/bin/bash
echo "I am auto-discovered action"' > test-trust/.cupcake/actions/RULE-002.sh
chmod +x test-trust/.cupcake/actions/RULE-002.sh
```

## Test BEFORE Fix (Using Old Parser)

The old trust system would only see scripts in guidebook.yml:
- ✅ explicit_signal
- ❌ auto_signal.sh (NOT SEEN)
- ❌ RULE-002.sh (NOT SEEN)

## Test AFTER Fix (Using Engine Parser)

The fixed trust system should see ALL scripts:
- ✅ explicit_signal (from YAML)
- ✅ auto_signal (auto-discovered from signals/)
- ✅ RULE-002 (auto-discovered from actions/)
- ✅ on_any_denial (from YAML)
- ✅ RULE-001 (from YAML)

## Commands to Verify

```bash
# Build the fixed version
cargo build -p cupcake-cli

# Initialize trust (should find ALL scripts)
./target/debug/cupcake trust init --project-dir test-trust

# List trusted scripts
./target/debug/cupcake trust list --project-dir test-trust

# Check what's actually in directories
ls -la test-trust/.cupcake/signals/
ls -la test-trust/.cupcake/actions/
```

## Expected Output After Fix

```
📁 Scanning for scripts (guidebook.yml + auto-discovery)...
📜 Found 5 scripts to trust:
  - signals/explicit_signal
  - signals/auto_signal     <- This was missing before!
  - actions/on_any_denial
  - actions/RULE-001
  - actions/RULE-002        <- This was missing before!
✅ Trust initialized successfully
```

## Security Verification

The fix ensures:
1. Trust system sees ALL scripts the engine will execute
2. No script can run without being in the trust manifest
3. Auto-discovered scripts are properly hashed and verified
4. `on_any_denial` actions are included in trust