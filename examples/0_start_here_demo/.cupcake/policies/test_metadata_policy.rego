package cupcake.policies.test.metadata

import rego.v1

# METADATA
# scope: rule
# title: Test Policy for Metadata Verification
# authors: ["Test Author"]
# custom:
#   severity: HIGH
#   id: TEST-METADATA-001
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
#     required_signals: ["test_signal"]

# Test decision verbs using modern Rego v1.0 syntax and NEW_GUIDING_FINAL format

# Deny commands containing "test" for demonstration
deny contains decision if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	contains(input.tool_input.command, "test")

	decision := {
		"reason": "Commands containing 'test' are blocked by test policy",
		"severity": "HIGH",
		"rule_id": "TEST-METADATA-001",
	}
}

# Ask for confirmation on long commands
ask contains decision if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	count(input.tool_input.command) > 50

	decision := {
		"reason": "Long command detected - please review for safety",
		"severity": "MEDIUM",
		"rule_id": "TEST-METADATA-002",
	}
}

# Add context about test signals
add_context contains "This policy demonstrates signal integration" if {
	# Trust routing - we know this is PreToolUse:Bash from metadata
	input.signals.test_signal
}
