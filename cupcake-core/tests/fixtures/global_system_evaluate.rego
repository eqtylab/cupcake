package cupcake.global.system

import rego.v1

evaluate := {
    "halts": collect_verbs("halt"),
    "denials": collect_verbs("deny"),
    "blocks": collect_verbs("block"), 
    "asks": collect_verbs("ask"),
    "allow_overrides": collect_verbs("allow_override"),
    "add_context": collect_verbs("add_context")
}

default collect_verbs(_) := []

collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.global.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set  
    ]
    result := all_decisions
}