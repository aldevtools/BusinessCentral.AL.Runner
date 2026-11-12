WorksheetDraftCloseEmptyPart => {
    id: "worksheet-draft-close-empty-part",
    assumed: "Closing an edit-mode Worksheet with AutoSplitKey, DelayedInsert and MultipleNewLines saves its new line without rereading an untouched empty part.",
    tests: "AL Runner issue 7880, measured on BC 27.5.46862.56996, 28.5.54151.57550 and 29.0.54011.57566.",
    probe: "Corpus pull request 1344.",
}
