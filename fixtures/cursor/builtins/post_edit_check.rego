# METADATA
# scope: package
# title: Post-Edit Check - Builtin Policy (Cursor)
# authors: ["Cupcake Builtins"]
# custom:
#   severity: MEDIUM
#   id: BUILTIN-POST-EDIT-CHECK
#   routing:
#     required_events: ["afterFileEdit"]
package cupcake.policies.builtins.post_edit_check

import rego.v1

# Ask user to run validation after file edits
ask contains decision if {
    input.hook_event_name == "afterFileEdit"

    # Get validation command from builtin config
    validation_cmd := input.builtin_config.post_edit_check.validation_command

    # Get the file that was edited
    file_path := input.file_path

    decision := {
        "rule_id": "BUILTIN-POST-EDIT-CHECK",
        "reason": concat("", [
            "File ",
            file_path,
            " was edited. Validation recommended."
        ]),
        "question": concat("", [
            "Run validation command: ",
            validation_cmd,
            "?"
        ]),
        "severity": "MEDIUM"
    }
}
