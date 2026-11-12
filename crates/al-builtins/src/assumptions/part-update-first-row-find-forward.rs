PartUpdateFirstRowFindForward => {
    id: "part-update-first-row-find-forward",
    assumed: "A ListPart with OnFindRecord (no OnNextRecord) on a Document host, standing on its first row, whose field OnValidate runs CurrPage.Update(true) without UpdatePropagation Both, reads its rows again with OnFindRecord('=>'), OnAfterGetRecord to the end of the view, the first row again, OnFindRecord('='), OnAfterGetRecord and OnAfterGetCurrRecord: `F=>;R1;F=;R1;C1;` with one row, `F=>;R1;R2;R3;R4;R5;R1;F=;R1;C1;` with five. Built only for the reporter's pages and calls, one to five rows.",
    tests: "No Microsoft test: al-runner issue 7820 (the reporter's real-BC run of OneRowSetValueOnFirst and FiveRowsSetValueOnFirst on BC 29.1).",
    probe: "pages/parts/R7820PartUpdateFindForward.al: the repro's pages with 1, 2 and 5 rows, the part on its first row, on BC 27.5, 28.5 and 29.0.",
    awaiting: Awaiting {
        issues: &[7820],
        files: &["tests/al-language/pages/parts/R7820PartUpdateFindForward.al"],
        since: "2026-11-12",
    },
}
