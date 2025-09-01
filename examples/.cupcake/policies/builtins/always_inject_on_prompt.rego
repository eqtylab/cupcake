package cupcake.policies.builtins.always_inject_on_prompt

import rego.v1

# METADATA
# scope: rule
# title: Always Inject On Prompt - Builtin Policy
# authors: ["Cupcake Builtins"]
# custom:
#   severity: LOW
#   id: BUILTIN-INJECT-PROMPT
#   routing:
#     required_events: ["UserPromptSubmit"]

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
        "severity": "LOW"
    }
}

# Get all configured contexts from signals
get_all_contexts := contexts if {
    # In production, this would:
    # 1. Query signals like __builtin_prompt_context_0, __builtin_prompt_context_1, etc.
    # 2. Collect results from each signal
    # 3. Format appropriately
    
    # For demonstration, provide example contexts
    contexts := [
        "Project Guidelines: Follow SOLID principles and write comprehensive tests",
        "Current Status: Development environment - be careful with database changes",
        "Team Convention: All new features require unit tests with >80% coverage"
    ]
    
    # In real implementation, would execute signals like:
    # signal_results := [
    #     data.signals["__builtin_prompt_context_0"],
    #     data.signals["__builtin_prompt_context_1"],
    #     data.signals["__builtin_prompt_context_2"]
    # ]
    # 
    # contexts := [ctx | 
    #     some result in signal_results
    #     ctx := format_context(result)
    # ]
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