EmptyWorksheetFilledByAction => {
    id: "empty-worksheet-filled-by-action",
    assumed: "A Worksheet with the new-row line (AutoSplitKey, DelayedInsert) opened for edit on an empty view, after an action that stored three rows or more (up to a window) through another record variable without CurrPage.Update: the client reads each row of the view once, the first row twice more, runs OnAfterGetCurrRecord on it with a blank xRec and goes back to its new-row line; a later action first reads the new line. Pinned with two rows (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:573, :601).",
    tests: "No Microsoft test: a partner app's tests (page 7372 Bin Creation Worksheet: Calculate Bins, then Create Bins).",
    probe: "The Worksheet of RmWorksheetGoneRow.al SH_WsWithNew_ with three and with 30 rows stored by the first action.",
}
