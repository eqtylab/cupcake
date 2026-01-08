# METADATA
# scope: package
# title: Cupcake Execution Protection - Global Builtin Policy (Cursor)
# authors: ["Cupcake Global Builtins"]
# custom:
#   severity: CRITICAL
#   id: GLOBAL-BUILTIN-CUPCAKE-EXEC-PROTECTION
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.global.policies.builtins.cupcake_exec_protection

import rego.v1

# Block direct execution of the cupcake binary
# This prevents agents from manipulating the policy engine itself
halt contains decision if {
	input.hook_event_name == "beforeShellExecution"

	# Get command from Cursor's raw schema
	command := lower(input.command)

	# Check if executing cupcake
	executes_cupcake(command)

	decision := {
		"rule_id": "GLOBAL-BUILTIN-CUPCAKE-EXEC-PROTECTION",
		"reason": "Direct execution of cupcake binary is not permitted. This protects the policy engine from manipulation.",
		"severity": "CRITICAL",
	}
}

# Detect cupcake execution attempts
executes_cupcake(cmd) if {
	# Direct invocation
	cupcake_patterns := {
		"cupcake eval", "cupcake init", "cupcake verify",
		"cupcake validate", "cupcake inspect",
		"./cupcake", "/cupcake",
	}

	some pattern in cupcake_patterns
	contains(cmd, pattern)
}

executes_cupcake(cmd) if {
	# Path-based execution
	contains(cmd, "/bin/cupcake")
}

executes_cupcake(cmd) if {
	# Cargo run from cupcake source
	contains(cmd, "cargo")
	contains(cmd, "cupcake")
}
