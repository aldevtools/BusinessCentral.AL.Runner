TempReadOnlyListGoneMiddleRowLandsBefore => {
    id: "temp-read-only-list-gone-middle-row-lands-before",
    assumed: "A List with Editable = false over a temporary source table, opened by the test with OpenView, whose action deletes the row it stands on, the middle one of three, lands on the row before it (not the row after it), reads it as a move does (OnAfterGetCurrRecord twice) and a later Next goes to the row after (al-vm pages/temp_list_gone_row.rs). Only the probe's page: fields of Rec in a repeater, no other routine than these triggers, each with the probe's body: OnOpenPage fills Rec from a local record with FindSet, `Rec := Row; Rec.Insert()` and Next, then one call of a procedure of a codeunit page variable with text of constants and fields of Rec as argument (the open stands on the last row); one action whose OnAction is one such call, then `Rec.Delete()`, and that asks for no refresh; OnAfterGetCurrRecord and OnClosePage one such call each; a table with OnModify only, no extension and no subscriber; the cursor reached by GoToKey right after the open (a read of a control between them allowed), the view in the primary key without filters. Every other list over a temporary table whose action deletes the row it stands on while rows are left is refused.",
    tests: "No Microsoft test.",
    probe: "Open corpus PR 1367 (testpage/lifecycle/ReopenSameVariableState.al:469 TempListRowGone_CloseReopen_DeleteAgain, `cursor-1` R1, `del-1` `[del(R2) agcr(R1) agcr(R1)]`, `next-in-1` R3, the same on the second open) passes on every major of its CI run on 32b29320.",
    awaiting: Awaiting {
        issues: &[7940],
        files: &["tests/al-language/testpage/lifecycle/ReopenSameVariableState.al"],
        since: "2026-11-13",
    },
}
