RelationConstFieldQualified => {
    id: "relation-const-field-qualified",
    assumed: "A TableRelation condition on an Enum field written `const(Type::\"Item Attribute\")`, the member qualified with the condition's own field name, names the member as `const(\"Item Attribute\")` does.",
    tests: "No Microsoft test: al-runner issue 6677 (Rename of Item with a table whose relation has this form; the reporter's run on BC passes).",
    probe: "A conditional TableRelation `if (Type = const(Type::X)) T.F`: validation and rename follow the arm exactly as with `const(X)`. Corpus writer Y65.",
    awaiting: Awaiting {
        issues: &[6677],
        files: &["tests/al-language/record/lt-y65-relation-field-qualified/Y65RelationFieldQualified.al"],
        since: "2026-11-01",
    },
}
