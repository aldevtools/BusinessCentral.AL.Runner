RelationConstFieldQualified => {
    id: "relation-const-field-qualified",
    assumed: "A TableRelation condition written `const(Type::Parent)` (Enum field), `const(Kind::Parent)` (Option field) or `filter(Type::Parent)`, the member qualified with the condition's own field name, names the member as `const(Parent)` / `filter(Parent)` does. Measured on SaaS (newest major) by corpus writer Y65; not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6677 (Rename of Item with a table whose relation has this form; the reporter's run on BC passes).",
    probe: "Y65RenameConstRelation.al Y65_Rename_QualifiedConstConditions (corpus PR 1012).",
    awaiting: Awaiting {
        issues: &[6677],
        files: &["tests/al-language/record/lt-y65-rename-const-relation/Y65RenameConstRelation.al"],
        since: "2026-11-01",
    },
}
