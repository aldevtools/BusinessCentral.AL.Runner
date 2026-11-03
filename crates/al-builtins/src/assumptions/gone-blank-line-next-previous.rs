GoneBlankLineNextPrevious => {
    id: "gone-blank-line-next-previous",
    assumed: "A List with the new-row line, opened for edit, that lost its only row to an action stands on its blank line. Next() and Previous() on it run OnNewRecord and OnAfterGetCurrRecord once each and answer No; First() and Last() answer No and run nothing. First() running nothing is pinned (pages/lt-f3-pages/F3ListTests.al:189 DeleteOnlyRow_WithoutUpdate); the rest is assumed: the corpus pins only the total of one call each of First, Next, Last, Previous in that order, two pairs (pages/lt-y80-gone-rows/Y80GoneRows.al Y80_Gone_All_OpenEdit_DeleteAllUpdate `moves log`).",
    tests: "No Microsoft test.",
    probe: "The list of Y80_Gone_All_OpenEdit_DeleteAllUpdate with one move call per log: Next, Last and Previous each followed by a read of the log, and each alone after the action.",
}
