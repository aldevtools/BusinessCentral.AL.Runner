StaleOwnInsertTwoCommitsOtherInsert => {
    id: "stale-own-insert-two-commits-other-insert",
    assumed: "A stale buffer from the variable's own Insert, or its own Modify after it, made in the transaction before the last Commit or the one before it, is written whole (the stale buffer wins) when the transaction after the last Commit inserted another row of the table and wrote beside that Insert something other than Inserts and Modifies of other rows of the table (a Delete or a Rename, a write of another table). Pinned for an own Insert with and without the own Modify, one, two and three Commits, and Inserts and a Modify of other rows of the table as the only writes of the new transaction (record/lt-sb-stale-own-modify/SbStaleOwnModify.al:666-745, :2484-2580), which do not mark it.",
    tests: "No Microsoft test: a partner app's tests (Library - Manufacturing.RefreshProdOrder commits before and after Refresh Production Order).",
    probe: "The Y2_SH_ tests of SbStaleOwnModify.al with a Delete, a Rename of another row or an Insert into another table beside the Insert of another row.",
}
