# METADATA
# scope: package
# title: Always Inject On Prompt - Builtin Policy
# authors: ["Cupcake Builtins"]
# custom:
#   severity: LOW
#   id: BUILTIN-INJECT-PROMPT
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.policies.builtins.factory_always_inject_on_prompt

import rego.v1

# Inject configured context on every user prompt
# Note: add_context returns strings, not objects
add_context contains combined_context if {
	input.hook_event_name == "UserPromptSubmit"

	# Get all configured context items
	contexts := get_all_contexts
	count(contexts) > 0

	# Combine all contexts into a single string
	combined_context := concat("\n\n", contexts)
}

# Get all configured contexts from both static config and dynamic signals
get_all_contexts := contexts if {
	# 1. Get static contexts from builtin_config (configured strings in rulebook)
	static_contexts := get_static_contexts

	# 2. Get dynamic contexts from signals (commands and files)
	dynamic_contexts := get_dynamic_contexts

	# Combine both sources
	contexts := array.concat(static_contexts, dynamic_contexts)

	# Ensure we have at least one context
	count(contexts) > 0
} else := [] if {
	true
}

# Get static string contexts from builtin_config
get_static_contexts := contexts if {
	config := input.builtin_config.claude_code_always_inject_on_prompt
	contexts := config.static_contexts
} else := [] if {
	true
}

# Get dynamic contexts from signals (commands and files)
get_dynamic_contexts := contexts if {
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
} else := [] if {
	true
}

# Format context based on its source
format_context(value) := formatted if {
	# If it's a string, use it directly
	is_string(value)
	formatted := value
} else := formatted if {
	# If it's an object/array, format as JSON
	formatted := json.marshal(value)
}
