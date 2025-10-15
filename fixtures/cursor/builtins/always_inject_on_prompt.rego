# METADATA
# scope: package
# title: Always Inject On Prompt - Builtin Policy (Cursor)
# authors: ["Cupcake Builtins"]
# custom:
#   severity: LOW
#   id: BUILTIN-ALWAYS-INJECT-ON-PROMPT
#   routing:
#     required_events: ["beforeSubmitPrompt"]
package cupcake.policies.builtins.always_inject_on_prompt

import rego.v1

# Inject context on every user prompt submission
add_context contains context_message if {
    input.hook_event_name == "beforeSubmitPrompt"

    # Get the message from builtin config
    context_message := input.builtin_config.always_inject_on_prompt.message
}
