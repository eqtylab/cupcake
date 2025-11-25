# METADATA
# scope: package
# title: Always Inject On Prompt - Builtin Policy
# authors: ["Cupcake Builtins"]
# custom:
#   severity: LOW
#   id: BUILTIN-INJECT-PROMPT
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.policies.builtins.opencode_always_inject_on_prompt

import rego.v1

# Inject configured context on every user prompt
add_context contains decision if {
	input.hook_event_name == "UserPromptSubmit"

	# Get all configured context items
	contexts := get_all_contexts
	count(contexts) > 0

	# Combine all contexts
	combined_context := concat("\n\n", contexts)

	decision := {
		"rule_id": "BUILTIN-INJECT-PROMPT",
		"context": combined_context,
		"severity": "LOW",
	}
}

# Get all configured contexts from signals
get_all_contexts := contexts if {
	# Collect all builtin prompt context signals
	signal_results := [value |
		some key, value in input.signals
		startswith(key, "__builtin_prompt_context_")
	]

	# Format each context appropriately
	contexts := [ctx |
		some result in signal_results
		ctx := format_context(result)
	]

	# Ensure we have at least one context
	count(contexts) > 0
} else := []

# No signals available or no contexts configured

# Format context based on its source
format_context(value) := formatted if {
	# If it's a string, use it directly
	is_string(value)
	formatted := value
} else := formatted if {
	# If it's an object/array, format as JSON
	formatted := json.marshal(value)
}
