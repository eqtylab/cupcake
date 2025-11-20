# METADATA
# scope: package
# title: Cupcake Execution Protection - Global Builtin Policy
# authors: ["Cupcake Global Builtins"]
# custom:
#   severity: CRITICAL
#   id: GLOBAL-BUILTIN-CUPCAKE-EXEC
#   routing:
#     required_events: ["PreToolUse"]
package cupcake.global.policies.builtins.cupcake_exec_protection

import rego.v1

# Block direct execution of cupcake binary via Bash
halt contains decision if {
	input.hook_event_name == "PreToolUse"
	input.tool_name == "Bash"

	command := lower(input.tool_input.command)

	# Check for cupcake binary execution patterns
	attempts_cupcake_execution(command)

	decision := {
		"rule_id": "GLOBAL-BUILTIN-CUPCAKE-EXEC",
		"reason": get_block_message,
		"severity": "CRITICAL",
	}
}

# Check if command attempts to run cupcake binary
attempts_cupcake_execution(cmd) if {
	# Direct cupcake invocation
	cupcake_patterns := {
		"cupcake ", # Direct command
		"./cupcake ", # Relative path
		"/cupcake ", # Could be in PATH or absolute
		"cupcake\t", # Tab after command
		"\ncupcake ", # New line then cupcake
		";cupcake ", # After semicolon
		"&& cupcake ", # After &&
		"|| cupcake ", # After ||
		"| cupcake ", # After pipe
		"`cupcake ", # In backticks
		"$(cupcake ", # In command substitution
		"${cupcake", # In variable expansion
	}

	some pattern in cupcake_patterns
	contains(cmd, pattern)
}

attempts_cupcake_execution(cmd) if {
	# Cargo run patterns (development)
	cargo_patterns := {
		"cargo run --bin cupcake",
		"cargo r --bin cupcake",
		"cargo run -p cupcake",
		"cargo r -p cupcake",
		"cargo run --release --bin cupcake",
		"cargo build --bin cupcake",
		"cargo install cupcake",
	}

	some pattern in cargo_patterns
	contains(cmd, pattern)
}

attempts_cupcake_execution(cmd) if {
	# Target directory patterns (built binaries)
	target_patterns := {
		"target/release/cupcake",
		"target/debug/cupcake",
		"target/x86_64", # Cross-compilation targets
		"target/aarch64", # ARM targets
		"target/wasm", # WASM targets
	}

	some pattern in target_patterns
	contains(cmd, pattern)
	contains(cmd, "cupcake") # Ensure it's the cupcake binary
}

attempts_cupcake_execution(cmd) if {
	# Installation and PATH patterns
	install_patterns := {
		"~/.cargo/bin/cupcake",
		"/usr/local/bin/cupcake",
		"/usr/bin/cupcake",
		"/opt/cupcake",
		"brew install cupcake",
		"apt install cupcake",
		"yum install cupcake",
		"snap install cupcake",
	}

	some pattern in install_patterns
	contains(cmd, pattern)
}

attempts_cupcake_execution(cmd) if {
	# Script execution that might contain cupcake
	script_patterns := {
		"sh ",
		"bash ",
		"zsh ",
		"source ",
		". ",
	}

	some script_cmd in script_patterns
	startswith(cmd, script_cmd)

	# Check if the script name suggests cupcake
	script_indicators := {
		"cupcake",
		"init-cupcake",
		"setup-cupcake",
		"run-cupcake",
	}

	some indicator in script_indicators
	contains(lower(cmd), indicator)
}

attempts_cupcake_execution(cmd) if {
	# Check for "cupcake" anywhere after eval/exec (obfuscation attempts)
	obfuscation_prefixes := {"eval ", "exec ", "alias ", "function "}

	some prefix in obfuscation_prefixes
	contains(cmd, prefix)
	contains(cmd, "cupcake")
}

# Get the configured block message or use default
get_block_message := msg if {
	msg := data.__builtin_cupcake_exec_message
} else := "Direct execution of Cupcake binary is not permitted"
