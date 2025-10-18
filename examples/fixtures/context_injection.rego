# METADATA
# scope: package
# title: Context Injection Policy
# description: Demonstrates adding helpful context to user prompts
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.policies.context_injection

import rego.v1

# Add project context to coding questions
add_context contains context_msg if {
    input.hook_event_name == "UserPromptSubmit"
    coding_keywords := {"implement", "code", "function", "bug", "error", "debug"}
    some keyword in coding_keywords
    contains(lower(input.prompt), keyword)
    context_msg := "Context: This is a Cupcake policy engine project using Rust and Rego. Current architecture uses the Hybrid Model with Rego for policies and Rust for routing/synthesis."
}

# Add reminder about testing when making changes
add_context contains context_msg if {
    input.hook_event_name == "UserPromptSubmit"
    change_keywords := {"modify", "update", "change", "fix", "add"}
    some keyword in change_keywords
    contains(lower(input.prompt), keyword)
    context_msg := "Reminder: Run tests with 'cargo test --features deterministic-tests' after making changes."
}