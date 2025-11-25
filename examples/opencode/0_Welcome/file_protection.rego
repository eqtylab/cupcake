# METADATA
# scope: package
# title: File Protection Policy
# description: Protect sensitive files and directories from modification
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Edit", "Write"]
package cupcake.policies.opencode.file_protection

import rego.v1

# Protect .env files from being edited
deny contains decision if {
	input.tool_name == "Edit"
	file_path := input.tool_input.filePath

	# Check if this is a .env file
	endswith(file_path, ".env")

	decision := {
		"rule_id": "ENV_FILE_PROTECTION",
		"reason": concat("", ["Attempted to edit .env file: ", file_path, ". Direct editing of .env files can expose secrets. Use a secure secrets management system."]),
		"severity": "HIGH",
	}
}

# Protect .env files from being created/written
deny contains decision if {
	input.tool_name == "Write"
	file_path := input.tool_input.filePath

	# Check if this is a .env file
	endswith(file_path, ".env")

	decision := {
		"rule_id": "ENV_FILE_WRITE_PROTECTION",
		"reason": concat("", ["Attempted to create/write .env file: ", file_path, ". Creating .env files via AI can expose secrets in logs and history. Use a secure secrets management system."]),
		"severity": "HIGH",
	}
}

# Protect configuration files in production
deny contains decision if {
	input.tool_name == "Write"
	file_path := input.tool_input.filePath

	# Check if we're writing to a config file
	protected_patterns := ["/etc/", ".config", "config.json", "config.yml"]
	some pattern in protected_patterns
	contains(file_path, pattern)

	decision := {
		"rule_id": "CONFIG_FILE_PROTECTION",
		"reason": concat("", ["Attempted to write to protected configuration file: ", file_path, ". Configuration changes should go through proper review and deployment processes."]),
		"severity": "HIGH",
	}
}

# Warn when modifying package.json dependencies
ask contains decision if {
	input.tool_name == "Edit"
	file_path := input.tool_input.filePath

	endswith(file_path, "package.json")

	# Check if modifying dependencies section
	old_string := input.tool_input.oldString
	new_string := input.tool_input.newString

	contains(old_string, "dependencies")
	contains(new_string, "dependencies")

	old_string != new_string

	decision := {
		"rule_id": "PACKAGE_JSON_DEPS",
		"reason": "You are modifying package.json dependencies. This can affect the entire project and should be reviewed carefully.",
		"question": "Have you reviewed these dependency changes?",
		"severity": "MEDIUM",
	}
}
