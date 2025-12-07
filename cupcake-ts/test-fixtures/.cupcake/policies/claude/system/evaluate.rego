package cupcake.system

import rego.v1

# METADATA
# scope: document
# title: System Aggregation Entrypoint for Tests
# authors: ["Cupcake Test Suite"]
# custom:
#   description: "Test fixture - aggregates all decision verbs from test policies"
#   entrypoint: true
#   routing:
#     required_events: []
#     required_tools: []

# The single entrypoint for test policies.
# This is REQUIRED for Cupcake to function properly.
evaluate := decision_set if {
    decision_set := {
        "halts": collect_verbs("halt"),
        "denials": collect_verbs("deny"),
        "blocks": collect_verbs("block"),
        "asks": collect_verbs("ask"),
        "modifications": collect_verbs("modify"),
        "add_context": collect_verbs("add_context")
    }
}

# Helper function to collect all decisions for a specific verb type.
collect_verbs(verb_name) := result if {
    # Collect all matching verb sets from the policy tree
    verb_sets := [value |
        walk(data.cupcake.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    
    # Flatten all sets into a single array
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]
    
    result := all_decisions
}

# Default to empty arrays if no decisions found
default collect_verbs(_) := []