ReadOnlyTempListGoneOnlyRow => {
    id: "read-only-temp-list-gone-only-row",
    assumed: "A List with Editable = false over a temporary source, without the new-row line, that lost its only row to an action: run modally it stands on a blank Rec and raises OnNewRecord and OnAfterGetCurrRecord once; opened by a TestPage for view or edit it raises nothing; no row is shown (real BC values that corpus writer Y66 measured, not on corpus master yet).",
    tests: "No Microsoft test: al-runner issue 6664, a filter action that deletes every row of a temporary list and refills the matching ones, possibly none.",
    probe: "Y66TempListGoneRow.al:235-460: RunModal, LookupMode, OpenView and OpenEdit, descending and ascending, with and without CurrPage.Update, then a second action and a refill.",
    awaiting: Awaiting {
        issues: &[6664],
        files: &["tests/al-language/pages/lt-y66-temp-list-gone-row/Y66TempListGoneRow.al"],
        since: "2026-11-01",
    },
}
