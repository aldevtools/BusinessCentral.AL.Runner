ReadOnlyViewListGoneOnlyRow => {
    id: "read-only-view-list-gone-only-row",
    assumed: "A List page with Editable = false, opened for view, without the new-row line, that lost its only row to an action raises no trigger after the action and shows no row, as a List with the line opened for view does (pages/W1Update.al:534). Pinned with InsertAllowed, DeleteAllowed and ModifyAllowed false and an action that calls CurrPage.Update(false) (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:481); with DeleteAllowed or ModifyAllowed left true it is assumed.",
    tests: "No Microsoft test: a partner app's tests (a \"Retry Now\" action on an error log list).",
    probe: "The List of RmWorksheetGoneRow.al SH_ListViewNotEditable_ with the Allowed properties left true, without CurrPage.Update, and with a second action.",
}
