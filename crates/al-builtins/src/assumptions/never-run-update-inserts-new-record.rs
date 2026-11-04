NeverRunUpdateInsertsNewRecord => {
    id: "never-run-update-inserts-new-record",
    assumed: "`CurrPage.Update(true)` or `Update()` of a Page variable that never ran, after its code filled a new record of the page's source table (AutoSplitKey list): the record is inserted, AutoSplitKey numbers it 10000 on an empty view and the previous line + 10000 after it, OnInsertRecord gets BelowxRec No for the first and Yes for the next, then the table's OnInsert runs.",
    tests: "Base Application page \"Table Filter\" (SetSourceTable fills its lines this way), reached by an ISV test app (al-runner issue 6951).",
    probe: "Pinned on corpus master by PR 1133 (format-evaluate/tablefilter-neutral/TableFilterEnumForms.al UpdateNeverRunPage_NewRecord_UpdateTrue, _UpdateFalse, _UpdateNoArgument, _NoUpdate); not yet in the runner's corpus pin.",
    awaiting: Awaiting {
        issues: &[6951],
        files: &["tests/al-language/format-evaluate/tablefilter-neutral/TableFilterEnumForms.al"],
        since: "2026-11-05",
    },
}
