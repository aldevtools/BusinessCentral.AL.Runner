SubpagelinkBlankRecordId => {
    id: "subpagelink-blank-record-id",
    assumed: "`SubPageLink = <RecordId field> = const()` selects the child rows whose RecordId is blank. Pinned on real BC by corpus pull request 1199 (pages/subpagelink-empty-term/TestSubPageLinkRecordId.al:53, not yet on corpus master). Any other text filter on a RecordId field stays refused.",
    tests: "No Microsoft test: al-runner issue 7267 (the reporter's real-BC run of SubPageLink_BlankRecordId_SelectsBlankRow on the three supported majors, corpus pull request 1199).",
    probe: "pages/subpagelink-empty-term/TestSubPageLinkRecordId.al once merged.",
    awaiting: Awaiting {
        issues: &[7267],
        files: &["tests/al-language/pages/subpagelink-empty-term/TestSubPageLinkRecordId.al"],
        since: "2026-11-08",
    },
}
