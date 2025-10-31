# METADATA
# scope: package
# title: Sensitive Data Protection - Global Builtin Policy (Cursor)
# authors: ["Cupcake Global Builtins"]
# custom:
#   severity: CRITICAL
#   id: GLOBAL-BUILTIN-SENSITIVE-DATA-PROTECTION
#   routing:
#     required_events: ["beforeReadFile", "beforeShellExecution"]
package cupcake.global.policies.builtins.sensitive_data_protection

import rego.v1

# Block reading of sensitive credential and key files
halt contains decision if {
	input.hook_event_name == "beforeReadFile"

	# Get file path from Cursor's raw schema
	# TOB-4 fix: Use canonical path (always provided by Rust preprocessing)
	file_path := lower(input.resolved_file_path)

	# Check if accessing sensitive data
	is_sensitive_path(file_path)

	decision := {
		"rule_id": "GLOBAL-BUILTIN-SENSITIVE-DATA-PROTECTION",
		"reason": concat("", ["Access to sensitive data blocked: ", input.file_path]),
		"severity": "CRITICAL",
	}
}

# Block shell commands that attempt to read sensitive files
halt contains decision if {
	input.hook_event_name == "beforeShellExecution"

	command := lower(input.command)

	# Check if command tries to access sensitive data
	accesses_sensitive_data(command)

	decision := {
		"rule_id": "GLOBAL-BUILTIN-SENSITIVE-DATA-PROTECTION",
		"reason": "Command blocked - attempts to access sensitive data",
		"severity": "CRITICAL",
	}
}

# Check if path is sensitive
is_sensitive_path(path) if {
	# SSH keys and config
	sensitive_patterns := {
		".ssh/id_", ".ssh/config", ".ssh/known_hosts",
		".ssh/authorized_keys", "/.ssh/",
	}

	some pattern in sensitive_patterns
	contains(path, pattern)
}

is_sensitive_path(path) if {
	# GPG/PGP keys
	gpg_patterns := {
		".gnupg/", "/.pgp/", ".pgp/private", "secring.gpg",
	}

	some pattern in gpg_patterns
	contains(path, pattern)
}

is_sensitive_path(path) if {
	# AWS credentials
	aws_patterns := {
		".aws/credentials", ".aws/config",
	}

	some pattern in aws_patterns
	contains(path, pattern)
}

is_sensitive_path(path) if {
	# Environment files with secrets
	env_patterns := {
		".env", ".env.local", ".env.production",
		"credentials.json", "service-account.json",
		"key.json", "private-key.json",
	}

	some pattern in env_patterns
	contains(path, pattern)
}

is_sensitive_path(path) if {
	# Docker and Kubernetes secrets
	k8s_patterns := {
		".kube/config", ".docker/config.json",
		"/run/secrets/", "secret.yaml",
	}

	some pattern in k8s_patterns
	contains(path, pattern)
}

is_sensitive_path(path) if {
	# Browser data
	browser_patterns := {
		"/cookies.sqlite", "/login data", "/web data",
		"chrome/default/", "firefox/", "safari/",
	}

	some pattern in browser_patterns
	contains(path, pattern)
}

is_sensitive_path(path) if {
	# Password managers
	password_patterns := {
		"1password", "lastpass", "keepass", "bitwarden",
		".password-store/",
	}

	some pattern in password_patterns
	contains(path, pattern)
}

# Check if command accesses sensitive data
accesses_sensitive_data(cmd) if {
	# Common sensitive file patterns
	sensitive_indicators := {
		".ssh/id_", ".ssh/config", ".gnupg/",
		".aws/credentials", ".env", "credentials.json",
		".kube/config", ".docker/config.json",
		"password", "secret", "token", "api_key",
	}

	some indicator in sensitive_indicators
	contains(cmd, indicator)
}

accesses_sensitive_data(cmd) if {
	# Commands that dump credentials
	credential_dump_patterns := {
		"security find-generic-password",
		"security find-internet-password",
		"keyring get", "secret-tool lookup",
		"cat ~/.ssh/", "cat ~/.gnupg/",
	}

	some pattern in credential_dump_patterns
	contains(cmd, pattern)
}
