# METADATA
# scope: package
# title: Sensitive Data Protection - Global Builtin Policy
# authors: ["Cupcake Global Builtins"]
# custom:
#   severity: HIGH
#   id: GLOBAL-BUILTIN-SENSITIVE-DATA
#   routing:
#     required_events: ["PreToolUse"]
package cupcake.global.policies.builtins.sensitive_data_protection

import rego.v1

# Block READ operations on sensitive files (credentials, secrets, keys)
deny contains decision if {
	input.hook_event_name == "PreToolUse"

	# Reading tools
	read_tools := {"Read", "Grep", "WebFetch"}
	input.tool_name in read_tools

	# Get the file path from tool input
	file_path := get_file_path_from_tool_input
	file_path != ""

	# Check if file appears to contain sensitive data
	is_sensitive_file(file_path)

	decision := {
		"rule_id": "GLOBAL-BUILTIN-SENSITIVE-DATA",
		"reason": concat("", ["Blocked access to potentially sensitive file: ", mask_path(file_path)]),
		"severity": "HIGH",
	}
}

# Block Glob patterns that could discover sensitive files
deny contains decision if {
	input.hook_event_name == "PreToolUse"
	input.tool_name == "Glob"

	pattern := input.tool_input.pattern

	# Check if searching for sensitive patterns
	is_sensitive_pattern(lower(pattern))

	decision := {
		"rule_id": "GLOBAL-BUILTIN-SENSITIVE-DATA",
		"reason": "Blocked search pattern that could discover sensitive files",
		"severity": "HIGH",
	}
}

# Block Bash commands that could read sensitive files
deny contains decision if {
	input.hook_event_name == "PreToolUse"
	input.tool_name == "Bash"

	command := lower(input.tool_input.command)

	# Check if command is trying to read sensitive files
	reads_sensitive_data(command)

	decision := {
		"rule_id": "GLOBAL-BUILTIN-SENSITIVE-DATA",
		"reason": "Command blocked - attempts to access sensitive data",
		"severity": "HIGH",
	}
}

# Check if a file path indicates sensitive data
is_sensitive_file(path) if {
	lower_path := lower(path)

	# Environment and configuration files
	sensitive_patterns := {
		".env", # Environment files
		".env.local",
		".env.production",
		".env.development",
		".env.staging",
		"dotenv",
	}

	some pattern in sensitive_patterns
	contains(lower_path, pattern)
}

is_sensitive_file(path) if {
	lower_path := lower(path)

	# Files with sensitive keywords
	sensitive_keywords := {
		"credential",
		"secret",
		"token",
		"apikey",
		"api_key",
		"api-key",
		"password",
		"passwd",
		"private",
		"auth",
	}

	some keyword in sensitive_keywords
	contains(lower_path, keyword)
}

is_sensitive_file(path) if {
	lower_path := lower(path)

	# Certificate and key files
	crypto_extensions := {
		".pem",
		".key",
		".p12",
		".pfx",
		".cer",
		".crt",
		".jks", # Java keystore
		".keystore",
		".ppk", # PuTTY private key
	}

	some ext in crypto_extensions
	endswith(lower_path, ext)
}

is_sensitive_file(path) if {
	lower_path := lower(path)

	# SSH keys
	ssh_patterns := {
		"id_rsa",
		"id_dsa",
		"id_ecdsa",
		"id_ed25519",
		".ssh/config",
		"known_hosts",
		"authorized_keys",
	}

	some pattern in ssh_patterns
	contains(lower_path, pattern)
}

is_sensitive_file(path) if {
	lower_path := lower(path)

	# Cloud provider credentials
	cloud_patterns := {
		".aws/credentials",
		".aws/config",
		".gcloud/",
		".azure/",
		".kube/config",
		"kubeconfig",
		".docker/config.json",
		".dockercfg",
	}

	some pattern in cloud_patterns
	contains(lower_path, pattern)
}

is_sensitive_file(path) if {
	lower_path := lower(path)

	# Package manager configs with tokens
	package_configs := {
		".npmrc",
		".pypirc",
		".gem/credentials",
		".m2/settings.xml",
		".gradle/gradle.properties",
		"nuget.config",
		".cargo/credentials",
	}

	some pattern in package_configs
	contains(lower_path, pattern)
}

is_sensitive_file(path) if {
	lower_path := lower(path)

	# Database and session files
	data_patterns := {
		".sqlite",
		".db",
		"database.yml",
		"database.json",
		"connection.json",
		"cookie",
		"session",
		".htpasswd",
		"wp-config.php", # WordPress config
	}

	some pattern in data_patterns
	contains(lower_path, pattern)
}

is_sensitive_file(path) if {
	lower_path := lower(path)

	# Git and VCS sensitive files
	vcs_patterns := {
		".git-credentials",
		".netrc",
		".gitconfig",
		".hgrc",
	}

	some pattern in vcs_patterns
	contains(lower_path, pattern)
}

# Check if glob pattern is searching for sensitive files
is_sensitive_pattern(pattern) if {
	sensitive_globs := {
		"*secret*",
		"*credential*",
		"*token*",
		"*password*",
		"*.env*",
		"*.key",
		"*.pem",
		"*apikey*",
		"*api_key*",
		"id_*",
		"*.sqlite",
		"*.db",
	}

	some glob in sensitive_globs
	contains(pattern, trim(glob, "*"))
}

# Check if bash command reads sensitive data
reads_sensitive_data(cmd) if {
	# Common read commands
	read_commands := {"cat", "less", "more", "head", "tail", "grep", "awk", "sed"}

	# Sensitive file indicators
	sensitive_indicators := {
		".env", "credential", "secret", "token", "password",
		"apikey", "api_key", ".pem", ".key", "id_rsa",
		".aws/", ".ssh/", ".npmrc", ".pypirc", "cookie",
	}

	# Check if command uses a read command AND references sensitive files
	some read_cmd in read_commands
	contains(cmd, read_cmd)

	some indicator in sensitive_indicators
	contains(cmd, indicator)
}

reads_sensitive_data(cmd) if {
	# Direct attempts to dump credential stores
	credential_commands := {
		"security find-generic-password", # macOS keychain
		"security dump-keychain", # macOS keychain dump
		"gpg --export", # GPG key export
		"ssh-add -l", # List SSH keys
		"aws configure get", # AWS credentials
		"gcloud auth print-access-token", # GCloud token
		"docker login", # Docker credentials
		"git config --get", # Git config values
	}

	some cred_cmd in credential_commands
	contains(cmd, cred_cmd)
}

# Mask sensitive path for logging (show only file type)
mask_path(path) := masked if {
	contains(lower(path), "key")
	masked := "[REDACTED-KEY-FILE]"
} else := masked if {
	contains(lower(path), "secret")
	masked := "[REDACTED-SECRET-FILE]"
} else := masked if {
	contains(lower(path), "credential")
	masked := "[REDACTED-CREDENTIAL-FILE]"
} else := masked if {
	contains(lower(path), "token")
	masked := "[REDACTED-TOKEN-FILE]"
} else := masked if {
	contains(lower(path), ".env")
	masked := "[REDACTED-ENV-FILE]"
} else := masked if {
	contains(lower(path), "password")
	masked := "[REDACTED-PASSWORD-FILE]"
} else := masked if {
	# Default: show only extension
	parts := split(path, "/")
	filename := parts[count(parts) - 1]
	contains(filename, ".")
	ext_parts := split(filename, ".")
	ext := ext_parts[count(ext_parts) - 1]
	masked := concat("", ["[SENSITIVE-FILE: *.", ext, "]"])
} else := "[SENSITIVE-FILE]"

# Extract file path from tool input
get_file_path_from_tool_input := path if {
	path := input.tool_input.file_path
} else := path if {
	path := input.tool_input.path
} else := path if {
	path := input.tool_input.url
	startswith(lower(path), "file://") # File URL
} else := path if {
	# For Grep, check the path parameter
	input.tool_name == "Grep"
	path := input.tool_input.path
} else := ""
