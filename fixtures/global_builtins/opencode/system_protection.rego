# METADATA
# scope: package
# title: System Protection - Global Builtin Policy
# authors: ["Cupcake Global Builtins"]
# custom:
#   severity: CRITICAL
#   id: GLOBAL-BUILTIN-SYSTEM-PROTECTION
#   routing:
#     required_events: ["PreToolUse"]
package cupcake.global.policies.builtins.system_protection

import rego.v1

# Block ANY file operations on critical system paths
halt contains decision if {
	input.hook_event_name == "PreToolUse"

	# Check for ANY file operation tools
	file_operation_tools := {
		"Edit", "Write", "MultiEdit", "NotebookEdit", # Writing tools
		"Read", # Reading tools
		"Grep", "Glob", # Search/listing tools
		"WebFetch", # Could use file:// URLs
		"Task", # Could spawn agent to bypass
	}
	input.tool_name in file_operation_tools

	# Get the file path from tool input
	# TOB-4 fix: Use canonical path (always provided by Rust preprocessing)
	file_path := input.resolved_file_path
	file_path != null

	# Check if targeting protected system path
	targets_system_path(file_path)

	decision := {
		"rule_id": "GLOBAL-BUILTIN-SYSTEM-PROTECTION",
		"reason": concat("", ["Access to critical system path blocked: ", file_path]),
		"severity": "CRITICAL",
	}
}

# Block Bash commands that could access system paths
halt contains decision if {
	input.hook_event_name == "PreToolUse"
	input.tool_name == "Bash"

	# Check if command references protected system paths
	command := lower(input.tool_input.command)
	references_system_path(command)

	decision := {
		"rule_id": "GLOBAL-BUILTIN-SYSTEM-PROTECTION",
		"reason": "Command blocked - references critical system paths",
		"severity": "CRITICAL",
	}
}

# Check if path targets critical system directories
targets_system_path(path) if {
	lower_path := lower(path)

	# Unix/Linux/macOS critical paths
	critical_prefixes := {
		"/etc/", # System configuration
		"/system/", # macOS system files
		"/usr/bin/", # System binaries
		"/usr/sbin/", # System admin binaries
		"/bin/", # Essential binaries
		"/sbin/", # System binaries
		"/boot/", # Boot files
		"/lib/", # System libraries
		"/lib64/", # 64-bit libraries
		"/sys/", # Kernel interfaces
		"/proc/", # Process information
		"/dev/", # Device files
		"/root/", # Root user home
		"/var/log/secure", # Security logs
		"/var/log/auth", # Auth logs
	}

	some prefix in critical_prefixes
	startswith(lower_path, prefix)
}

targets_system_path(path) if {
	lower_path := lower(path)

	# macOS specific sensitive paths
	mac_sensitive := {
		"/library/launchagents/", # Startup items
		"/library/launchdaemons/", # System daemons
		"/library/preferences/", # System preferences
		"/private/etc/", # Private etc
		"/private/var/", # Private var
	}

	some prefix in mac_sensitive
	startswith(lower_path, prefix)
}

targets_system_path(path) if {
	# User home sensitive paths (expand ~)
	lower_path := lower(path)

	home_sensitive := {
		"~/.ssh/", # SSH config and keys
		"~/library/launchagents/", # User startup items (macOS)
		"~/.gnupg/", # GPG keys
		"~/.config/", # User configs
		"~/.local/share/keyrings/", # Keyrings
	}

	some pattern in home_sensitive
	startswith(lower_path, pattern)
}

targets_system_path(path) if {
	# Windows critical paths
	lower_path := lower(path)

	windows_critical := {
		"c:\\windows\\", # Windows directory
		"c:\\program files\\", # Program files
		"c:\\program files (x86)\\", # 32-bit programs
		"c:\\programdata\\", # Program data
		"c:\\users\\all users\\", # All users data
		"c:\\bootmgr", # Boot manager
		"%systemroot%", # System root
		"%windir%", # Windows directory
	}

	some prefix in windows_critical
	startswith(lower_path, prefix)
}

# Check if bash command references system paths
references_system_path(cmd) if {
	system_indicators := {
		"/etc/", "/system/", "/usr/bin/", "/usr/sbin/",
		"/boot/", "/lib/", "/sys/", "/proc/", "/dev/",
		"~/.ssh/", "/library/launch", "c:\\windows\\",
		"%systemroot%", "%windir%", "/private/etc/",
		"/private/var/", "sudo ", "doas ",
	}

	some indicator in system_indicators
	contains(cmd, indicator)
}

# Extract file path from tool input
get_file_path_from_tool_input := path if {
	path := input.tool_input.file_path
} else := path if {
	path := input.tool_input.path
} else := path if {
	path := input.tool_input.pattern
} else := path if {
	path := input.tool_input.url
} else := path if {
	path := input.tool_input.notebook_path
} else := path if {
	# MultiEdit tool
	some edit in input.tool_input.edits
	path := edit.file_path
	targets_system_path(path) # Return first system path found
} else := ""
