TreeWorksheetGoneOnlyRowBlankRec => {
    id: "tree-worksheet-gone-only-row-blank-rec",
    assumed: "A Worksheet without the new-row line whose repeater shows its rows as a tree (ShowAsTree, IndentationColumn), opened for edit, that lost its only row to an action stands on a blank Rec as the same Worksheet without the tree does: OnNewRecord and OnAfterGetCurrRecord once, at that action and at a later one. Pinned for a page with InsertAllowed, DeleteAllowed and ModifyAllowed false (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:509, :535); with DeleteAllowed or ModifyAllowed left true it is assumed.",
    tests: "No Microsoft test: a partner app's tests (a check worksheet's finish action that recalculates to no rows).",
    probe: "The tree Worksheet of RmWorksheetGoneRow.al with DeleteAllowed and ModifyAllowed left true, with an action that calls CurrPage.Update(false), and over a table that is not temporary.",
}
