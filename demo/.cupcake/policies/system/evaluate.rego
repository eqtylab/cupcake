package cupcake.system

import rego.v1

# METADATA
# scope: rule
# title: System Aggregation Policy
# authors: ["Cupcake Engine"]

# Collect all decision verbs from the policy hierarchy
# Uses walk() for automatic policy discovery

halts := collect_verbs("halt")
denies := collect_verbs("deny") 
blocks := collect_verbs("block")
asks := collect_verbs("ask")
allow_overrides := collect_verbs("allow_override")
add_context := collect_verbs("add_context")

# Single evaluation entrypoint for the engine
evaluate := {
    "halts": halts,
    "denies": denies,
    "blocks": blocks,
    "asks": asks,
    "allow_overrides": allow_overrides,
    "add_context": add_context
}

# Collect all instances of a decision verb across all policies
collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]
    result := all_decisions
}