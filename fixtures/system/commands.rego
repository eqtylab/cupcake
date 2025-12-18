# METADATA
# scope: package
# description: Helper functions for secure command analysis
package cupcake.system.commands

import rego.v1

# Check if command contains a specific verb with proper word boundary anchoring
# This prevents bypass via extra whitespace: "git  commit" or "  git commit"
has_verb(command, verb) if {
	pattern := concat("", ["(^|\\s)", verb, "(\\s|$)"])
	regex.match(pattern, command)
}

# Check if command contains ANY of the dangerous verbs from a set
# More efficient than checking each verb individually in policy code
has_dangerous_verb(command, verb_set) if {
	some verb in verb_set
	has_verb(command, verb)
}

# Detect symlink creation commands
# Matches: ln -s, ln -sf, ln -s -f, etc.
creates_symlink(command) if {
	has_verb(command, "ln")
	contains(command, "-s")
}

# Check if symlink command involves a protected path
# IMPORTANT: Checks BOTH source and target (addresses TOB-EQTY-LAB-CUPCAKE-4)
# Blocks: ln -s .cupcake foo AND ln -s foo .cupcake
symlink_involves_path(command, protected_path) if {
	creates_symlink(command)
	contains(command, protected_path)
}

# Detect output redirection operators that could bypass file protection
# Matches: >, >>, |, tee
has_output_redirect(command) if {
	redirect_patterns := [
		`\s>\s`, # stdout redirect
		`\s>>\s`, # stdout append
		`\s\|\s`, # pipe
		`(^|\s)tee(\s|$)`, # tee command
	]
	some pattern in redirect_patterns
	regex.match(pattern, command)
}

