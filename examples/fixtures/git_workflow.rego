# METADATA
# scope: package
# title: Git Workflow Policy
# description: Enforces git best practices and prevents risky operations
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.git_workflow

import rego.v1

# Ask before force pushing
ask contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "git push")
    contains(input.tool_input.command, "--force")
    decision := {
        "rule_id": "GIT-001",
        "reason": "Force push detected - this can overwrite remote history",
        "question": "Are you sure you want to force push? This is potentially destructive.",
        "severity": "HIGH"
    }
}

# Block commits to main/master without confirmation
ask contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "git commit")
    # This would typically check current branch via signal
    decision := {
        "rule_id": "GIT-002", 
        "reason": "Committing changes - please confirm",
        "question": "Ready to commit these changes?",
        "severity": "MEDIUM"
    }
}