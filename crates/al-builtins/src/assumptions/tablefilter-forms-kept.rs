TablefilterFormsKept => {
    id: "tablefilter-forms-kept",
    assumed: "Evaluate into a TableFilter keeps these conditions as written (Code values in capitals): Text `''`, `<>''`, `?*` and a value in double quotes that holds a comma; Decimal `<>0`, `\">=100000\"` and `\">=100,000\"`; Code `CORP*` and `<>CORP*`; Boolean `Yes`; an Enum ordinal no member has. Pinned: the stored text, fed to the Base Application page Table Filter and FieldRef.SetFilter, selects the rows the tests name, and for the undeclared Enum ordinal the page gives the filter back as `10` (format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:688-709 and :773-895). The stored text itself and the neighbouring forms are not pinned.",
    tests: "No Microsoft test: al-runner issue 7188 (the reporter's real-BC run of the fourteen TableFilter_Evaluate_* tests on the three supported majors, corpus pull request 1165).",
    probe: "format-evaluate/tablefilter-neutral/R7188StoredForms.al: what Evaluate returns and Format gives back for each of these conditions, and for `<>5`, `>=100,000` without quotes, `x*` on Text and `no` on Boolean.",
    awaiting: Awaiting {
        issues: &[7188],
        files: &["tests/al-language/format-evaluate/tablefilter-neutral/R7188StoredForms.al"],
        since: "2026-11-06",
    },
}
