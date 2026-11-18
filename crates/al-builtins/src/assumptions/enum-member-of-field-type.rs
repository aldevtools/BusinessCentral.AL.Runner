EnumMemberOfFieldType => {
    id: "enum-member-of-field-type",
    assumed: "`Rec.Field::Member`, where Rec is a record and Field an Enum field, names a member of the field's own enum (the enum the file that declares the field names) when the file of the expression finds the enum's name twice, or finds another enum of that name in its own namespace or through a `using`. An assignment, a comparison, a `case` label and `.AsInteger()` on the member take the field's enum's value. Every other reading keeps main's answer: the file finds the field's enum; a bare field name `Field::Member` inside the table's own code; another enum of that name reached through the global namespace or the search in every namespace; a name the file does not find.",
    tests: "An outside reporter's repro passed 2/2 on Business Central 29.1 (al-runner issue 8317): a table in one namespace with an Enum field of an enum in another, an enum of the same name in a third namespace of the test app, and a test file with a `using` of the table's namespace only.",
    probe: "Corpus PR 1442 (open), tests/al-language/types/enum-option/EnumMemberFieldType_Tests.al: the field's enum and table in XApp A, an enum of the same name with other ordinals in the coverage app; `Line.Kind::Production` from a file that finds the name twice, from the twin's own namespace and through a `using` of it. Measured 3/3 green on a local Business Central 29.1 container.",
    awaiting: Awaiting {
        issues: &[8317],
        files: &["tests/al-language/types/enum-option/EnumMemberFieldType_Tests.al"],
        since: "2026-11-18",
    },
}
