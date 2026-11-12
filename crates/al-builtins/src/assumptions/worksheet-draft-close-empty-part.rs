WorksheetDraftCloseEmptyPart => {
    id: "worksheet-draft-close-empty-part",
    assumed: "Closing an edit-mode Worksheet with AutoSplitKey, DelayedInsert and MultipleNewLines saves its new line without rereading an untouched empty part.",
    tests: "AL Runner issue 7880, measured on all three supported BC majors. The issue records the exact builds.",
    probe: "Corpus pull request 1344.",
}
