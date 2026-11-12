WorksheetDraftCloseEmptyPart => {
    id: "worksheet-draft-close-empty-part",
    assumed: "Closing an edit-mode Worksheet with AutoSplitKey, DelayedInsert and MultipleNewLines saves its new line without rereading an untouched empty part.",
    tests: "Local neutral DraftPageParts.al Worksheet probe.",
    probe: "Add the Worksheet close with a new line and an empty linked factbox to the pinned corpus.",
}
