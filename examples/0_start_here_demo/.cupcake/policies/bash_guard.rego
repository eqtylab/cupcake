# METADATA
# scope: package
# title: Bash Security Guard Policy
# authors: ["Security Team"]
# custom:
#   severity: HIGH
#   id: BASH-001
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
#     required_signals: ["git_branch", "test_status"]
package cupcake.policies.bash_guard

import rego.v1

# Decision Verbs - Modern Rego v1.0 format for NEW_GUIDING_FINAL.md

# Halt: Immediate cessation for catastrophic commands
halt contains decision if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	contains(input.tool_input.command, "rm -rf /")

	decision := {
		"reason": "EMERGENCY HALT: 'rm -rf /' command would destroy the entire filesystem",
		"severity": "CRITICAL",
		"rule_id": "BASH-001-HALT",
	}
}

# Deny: High-risk commands requiring explicit approval
deny contains decision if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	contains(input.tool_input.command, "sudo")
	not contains(input.tool_input.command, "sudo -l") # Allow listing permissions

	decision := {
		"reason": "Sudo commands require explicit approval for security",
		"severity": "HIGH",
		"rule_id": "BASH-001-SUDO",
	}
}

deny contains decision if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	regex.match(`\b(rm|rmdir)\s+.*(-r|--recursive).*(-f|--force)`, input.tool_input.command)
	not contains(input.tool_input.command, "rm -rf /") # Caught by halt above

	decision := {
		"reason": "Recursive force deletion commands are high-risk operations",
		"severity": "HIGH",
		"rule_id": "BASH-001-FORCE-DELETE",
	}
}

# Enhanced rules using signal data
deny contains decision if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	contains(input.tool_input.command, "git push")
	input.signals.git_branch == "main" # Use signal data

	decision := {
		"reason": "Direct pushes to main branch are not allowed",
		"severity": "HIGH",
		"rule_id": "BASH-001-MAIN-PUSH",
	}
}

# Enhanced rule using complex signal data structures
deny contains decision if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	contains(input.tool_input.command, "kubectl apply")
	input.signals.test_status.coverage < 80 # Use numeric field from structured signal
	count(input.signals.test_status.failed_tests) > 0 # Use array length from structured signal

	decision := {
		"reason": sprintf(
			"Cannot deploy with low test coverage (%.1f%%) and %d failing tests",
			[input.signals.test_status.coverage, count(input.signals.test_status.failed_tests)],
		),
		"severity": "HIGH",
		"rule_id": "BASH-001-DEPLOY-COVERAGE",
	}
}

# Context: Helpful information for users
add_context contains "⚠️ Working in production environment - extra caution advised" if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	contains(input.cwd, "/prod")
}

add_context contains sprintf("📋 Current branch: %s", [input.signals.git_branch]) if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	input.signals.git_branch != "unknown" # String signal access
}

add_context contains sprintf(
	"🧪 Test status: %.1f%% coverage, %d failing tests",
	[input.signals.test_status.coverage, count(input.signals.test_status.failed_tests)],
) if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	input.signals.test_status # Structured signal access with multiple fields
}

# Ask: Potentially risky commands requiring user confirmation
ask contains decision if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	regex.match(`\b(wget|curl)\s+.*\|\s*(bash|sh)`, input.tool_input.command)

	decision := {
		"reason": "Downloading and executing scripts can be dangerous. Confirm this is safe?",
		"severity": "MEDIUM",
		"rule_id": "BASH-001-PIPE-EXEC",
	}
}

ask contains decision if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	contains(input.tool_input.command, "npm publish")
	input.signals.test_status.passing == false # Use structured signal data

	decision := {
		"reason": "Tests are failing. Are you sure you want to publish?",
		"severity": "MEDIUM",
		"rule_id": "BASH-001-PUBLISH-FAILING",
	}
}
