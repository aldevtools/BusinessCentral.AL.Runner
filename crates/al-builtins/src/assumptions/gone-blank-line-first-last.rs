GoneBlankLineFirstLast => {
    id: "gone-blank-line-first-last",
    assumed: "A List with the new-row line, opened for edit, that lost its only row to an action stands on its blank line. First() and Last() on it run OnNewRecord and OnAfterGetCurrRecord once each (the blank line becomes the current line again); Next() and Previous() answer No and run nothing. The corpus pins only the total of one call each of First, Next, Last, Previous in that order: two pairs (pages/lt-y80-gone-rows/Y80GoneRows.al Y80_Gone_All_OpenEdit_DeleteAllUpdate `moves log`).",
    tests: "No Microsoft test.",
    probe: "The list of Y80_Gone_All_OpenEdit_DeleteAllUpdate with one move call per log: First, Next, Last and Previous each followed by a read of the log, and each alone after the action.",
}
