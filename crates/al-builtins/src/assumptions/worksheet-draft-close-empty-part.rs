WorksheetDraftCloseEmptyPart => {
    id: "worksheet-draft-close-empty-part",
    assumed: "Closing an OpenEdit Worksheet with AutoSplitKey, DelayedInsert and MultipleNewLines, whose new line SetValue started on the empty page and whose key comes from OnNewRecord and AutoSplitKey (no key field shown), saves the line without rereading its untouched empty FactBox ListPart linked by one field. The Worksheet and the part have no page extension or subscriber.",
    tests: "AL Runner issue 7880 (reporter's repro CloseStoresWorksheetRowWithoutReadingEmptyPart, measured on all three supported BC majors; the issue records the exact builds).",
    probe: "Corpus pull request 1344.",
    awaiting: Awaiting {
        issues: &[7880],
        files: &["tests/al-language/pages/lifecycle/TestWorksheetDraftClose.al"],
        since: "2026-11-12",
    },
}
