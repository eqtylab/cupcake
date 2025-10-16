# METADATA
# scope: package
# title: Cursor Prompt Filtering
# custom:
#   routing:
#     required_events: ["beforeSubmitPrompt"]
package cupcake.policies.cursor.prompt_filter

import rego.v1

# Block prompts containing secrets
deny contains decision if {
    input.hook_event_name == "beforeSubmitPrompt"
    secret_patterns := ["password", "api_key", "secret", "token"]
    some pattern in secret_patterns
    contains(lower(input.prompt), pattern)

    # Check if it looks like an actual secret (long random string)
    regex.match(`[A-Za-z0-9]{20,}`, input.prompt)

    decision := {
        "rule_id": "CURSOR-PROMPT-001",
        "reason": "Potential secret detected in prompt",
        "severity": "HIGH"
    }
}
