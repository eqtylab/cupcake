# METADATA
# scope: package
# title: Git Workflow Enforcement
# description: Enforce git best practices and workflows
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.git_workflow

import rego.v1

# Require descriptive commit messages (not just "wip", "fix", etc.)
ask contains decision if {
    input.tool_name == "Bash"
    command := input.tool_input.command
    
    # Check if this is a git commit
    contains(command, "git commit")
    contains(command, "-m")
    
    # Extract commit message (simplified - in real policy would be more robust)
    lazy_messages := ["wip", "fix", "tmp", "test", "asdf"]
    some lazy in lazy_messages
    contains(lower(command), lazy)
    
    decision := {
        "rule_id": "GIT_LAZY_COMMIT",
        "reason": "Your commit message appears to be non-descriptive. Consider using a more meaningful message that explains what changed and why.",
        "question": "Do you want to proceed with this commit message?",
        "severity": "MEDIUM"
    }
}

# Warn before pushing to main
ask contains decision if {
    input.tool_name == "Bash"
    command := input.tool_input.command
    
    contains(command, "git push")
    contains(command, "main")
    
    decision := {
        "rule_id": "GIT_PUSH_MAIN",
        "reason": "You are pushing directly to the main branch. Consider using a feature branch and pull request workflow.",
        "question": "Are you sure you want to push to main?",
        "severity": "MEDIUM"
    }
}
