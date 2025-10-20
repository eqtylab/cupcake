# METADATA
# scope: package
# title: Git Block No-Verify - Builtin Policy (Cursor)
# authors: ["Cupcake Builtins"]
# custom:
#   severity: HIGH
#   id: BUILTIN-GIT-BLOCK-NO-VERIFY
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.builtins.git_block_no_verify

import rego.v1

import data.cupcake.helpers.commands

# Block git commands that bypass verification hooks
deny contains decision if {
	input.hook_event_name == "beforeShellExecution"

	# Get the command directly from input (Cursor's raw schema)
	command := lower(input.command)

	# Check if it's a git command with --no-verify flag
	contains_git_no_verify(command)

	decision := {
		"rule_id": "BUILTIN-GIT-BLOCK-NO-VERIFY",
		"reason": "Git operations with --no-verify are not permitted. Commit hooks must run for code quality and security checks.",
		"severity": "HIGH",
	}
}

# Uses helper library to prevent spacing bypass (TOB-EQTY-LAB-CUPCAKE-3)
# Check if command contains git with --no-verify flag
contains_git_no_verify(cmd) if {
	# Check for git commit with --no-verify
	commands.has_verb(cmd, "git")
	commands.has_verb(cmd, "commit")
	contains(cmd, "--no-verify")
}

contains_git_no_verify(cmd) if {
	# Check for git commit with -n (shorthand for --no-verify)
	commands.has_verb(cmd, "git")
	commands.has_verb(cmd, "commit")
	regex.match(`\s-[a-z]*n[a-z]*\s`, concat(" ", [cmd, " "])) # Matches -n, -an, -nm, etc.
}

contains_git_no_verify(cmd) if {
	# Check for git push with --no-verify
	commands.has_verb(cmd, "git")
	commands.has_verb(cmd, "push")
	contains(cmd, "--no-verify")
}

contains_git_no_verify(cmd) if {
	# Check for git merge with --no-verify
	commands.has_verb(cmd, "git")
	commands.has_verb(cmd, "merge")
	contains(cmd, "--no-verify")
}

# Also block attempts to disable hooks via config
deny contains decision if {
	input.hook_event_name == "beforeShellExecution"

	command := lower(input.command)

	# Check if trying to disable hooks via git config
	contains_hook_disable(command)

	decision := {
		"rule_id": "BUILTIN-GIT-BLOCK-NO-VERIFY",
		"reason": "Disabling git hooks is not permitted. Hooks are required for code quality and security.",
		"severity": "HIGH",
	}
}

contains_hook_disable(cmd) if {
	commands.has_verb(cmd, "git")
	commands.has_verb(cmd, "config")
	contains(cmd, "core.hooksPath")
	contains(cmd, "/dev/null")
}

contains_hook_disable(cmd) if {
	# Detect attempts to chmod hooks to non-executable
	commands.has_verb(cmd, "chmod")
	regex.match(`\.git/hooks`, cmd)
	regex.match(`-x|-[0-9]*0[0-9]*`, cmd) # Removing execute permission
}

contains_hook_disable(cmd) if {
	# Detect attempts to remove hook files
	contains(cmd, ".git/hooks")
	removal_cmds := {"rm", "unlink", "trash"}
	commands.has_dangerous_verb(cmd, removal_cmds)
}

contains_hook_disable(cmd) if {
	# Detect moving/renaming hooks to disable them
	commands.has_verb(cmd, "mv")
	contains(cmd, ".git/hooks")
}
