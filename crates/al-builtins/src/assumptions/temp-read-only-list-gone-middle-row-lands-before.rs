TempReadOnlyListGoneMiddleRowLandsBefore => {
    id: "temp-read-only-list-gone-middle-row-lands-before",
    assumed: "A List with Editable = false over a temporary source table, opened by the test with OpenView, whose action deletes the row it stands on, the middle one of three, lands on the row before it (not the row after it), reads it as a move does (OnAfterGetCurrRecord twice) and a later Next goes to the row after (al-vm pages/temp_list_gone_row.rs). Only the probe's page: OnOpenPage, OnAfterGetCurrRecord and OnClosePage its only triggers, fields of Rec in a repeater, one action with OnAction, a table with OnModify only, no extension and no subscriber; the cursor reached by GoToKey right after the open (a read of a control between them allowed), the view in the primary key without filters. Every other list over a temporary table whose action deletes the row it stands on while rows are left is refused.",
    tests: "No Microsoft test.",
    probe: "Open corpus PR 1367 (testpage/lifecycle/ReopenSameVariableState.al:469 TempListRowGone_CloseReopen_DeleteAgain, `cursor-1` R1, `del-1` `[del(R2) agcr(R1) agcr(R1)]`, `next-in-1` R3, the same on the second open) passes on BC 27, 28 and 29 (its CI on 32b29320 is green).",
    awaiting: Awaiting {
        issues: &[7940],
        files: &["tests/al-language/testpage/lifecycle/ReopenSameVariableState.al"],
        since: "2026-11-13",
    },
}
