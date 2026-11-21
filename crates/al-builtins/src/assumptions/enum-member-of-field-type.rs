EnumMemberOfFieldType => {
    id: "enum-member-of-field-type",
    assumed: "`Field::Member` on an Enum field names a member of the field's own enum (the enum the file that declares the field names) when the file of the expression finds another enum by the enum's name (in its own namespace, through a `using` or in the global namespace) or finds the name more than once. An assignment, a comparison, a `case` label and `.AsInteger()` on the member take the field's enum's value, also for a member an enum extension adds. In these files `Rec.Field` itself is of the field's enum type: a member only the file's enum has, the field into a variable or parameter of the file's enum, and a member of the file's enum into the field do not compile (AL0132, AL0122, AL0133). Taken only where the record is a local variable or parameter in a codeunit, table extension or page, or is Rec or the bare field name in a table extension or page, and the field's enum is in a namespace; in every other shape (a global variable, xRec, `this`, Rec of a codeunit with TableNo, a report, a `with` record, an array element, a call's result, a field enum of the global namespace) the routine that uses the field refuses (checker note AR1010). Where the file finds the field's enum itself, or no enum of that name, main's answer stays.",
    tests: "An outside reporter's repro passed 2/2 on Business Central 29.1 (al-runner issue 8317). The probe of corpus PR 1480 passed 12/12 on a local Business Central 29.1 OnPrem w1 container, with its must-not-compile cases 3290 to 3293 rejected by alc 30.0.",
    probe: "Corpus PR 1442 (merged, not pinned yet), tests/al-language/types/enum-option/EnumMemberFieldType_Tests.al:29, :59, :77: a file that finds the name twice, the twin's own namespace, a `using` of it; green 3/3 on every supported major in its CI. Corpus PR 1480 (merged at 3fd40c85, not pinned yet; green on every supported major in its CI), tests/al-language/types/enum-option/EnumMemberFieldTypeShapes_Tests.al: a temporary record :30, a member of an enum extension :41, a field of a table extension :58, a field enum of the global namespace :69 (refused here), Rec and the bare field in a table extension :80 and in a page :93, usings that find the field's enum and a twin :106, usings that find two twins :117, a twin of the global namespace :128; must-not-compile 3290 to 3293.",
    awaiting: Awaiting {
        issues: &[8317],
        files: &[
            "tests/al-language/types/enum-option/EnumMemberFieldType_Tests.al",
            "tests/al-language/types/enum-option/EnumMemberFieldTypeShapes_Tests.al",
        ],
        since: "2026-11-18",
    },
}
