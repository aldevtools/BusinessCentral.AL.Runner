CuegroupFieldValue => {
    id: "cuegroup-field-value",
    assumed: "TestPage Value() of a field control inside a cuegroup, read by its name or through TestPage.GetField: an Integer or a Decimal field shows its value as anywhere else, an Option or an Enum field its ordinal, and a Code, Boolean, Date, BigInteger, Time, DateTime, Guid or Duration field 0, whatever it holds (a Code that holds a number too). A Text field shows 0 too, but that is pinned (testpage/ABTests.al:747, pages/properties/ABLayouts.al:68) and runs without this entry. In a cuegroup with CuegroupLayout = Wide the same for Code and Integer (Decimal is pinned). Every other type refuses, and so does every other type in a wide cuegroup. Only Value() is changed: the other calls on such a control read as before. Main showed the field's own text.",
    tests: "No Microsoft test: al-runner issue 8845 (the reporter's repro, green on real Business Central 29.1, 3/3).",
    probe: "Corpus pull request 1538 (open), tests/al-language/testpage/GetFieldByControlId.al:742 (CueTypes_ValueByNameAndById: every type above by its name and by its id, the wide cuegroup's Code and Integer) and :653 (a Text field by both); on a local Business Central 29.1, 3 runs.",
    awaiting: Awaiting {
        issues: &[8845],
        files: &["tests/al-language/testpage/GetFieldByControlId.al"],
        since: "2026-11-23",
    },
}
