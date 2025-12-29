# METADATA
# scope: package

package cupcake.policies.python_package_install

import rego.v1

# Detect pip/pip3 install commands
is_pip_install if {
    regex.match(`(^|\s)(pip|pip3)\s+install\s`, input.tool_input.command)
}

# Detect uv install/add commands
is_uv_install if {
    regex.match(`(^|\s)uv\s+(pip\s+install|add)\s`, input.tool_input.command)
}

# Check if signal returned an error
signal_has_error if {
    input.signals.package_check.error
}

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    is_pip_install
    signal_has_error

    decision := {
        "rule_id": "pip_package_blocked",
        "reason": sprintf("Package installation blocked: %s", [input.signals.package_check.error]),
        "severity": "HIGH"
    }
}

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    is_uv_install
    signal_has_error

    decision := {
        "rule_id": "uv_package_blocked",
        "reason": sprintf("Package installation blocked: %s", [input.signals.package_check.error]),
        "severity": "HIGH"
    }
}
