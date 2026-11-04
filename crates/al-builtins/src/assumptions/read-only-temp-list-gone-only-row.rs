ReadOnlyTempListGoneOnlyRow => {
    id: "read-only-temp-list-gone-only-row",
    assumed: "A List with Editable = false over a temporary source, without the new-row line, that lost its only row to an action: run modally it stands on a blank Rec and raises OnNewRecord and OnAfterGetCurrRecord once; opened by a TestPage for view or edit it raises nothing; no row is shown (real BC values that corpus writer Y66 measured, not on corpus master yet).",
    tests: "No Microsoft test: an outside report, a filter action that deletes every row of a temporary list and refills the matching ones, possibly none.",
    probe: "Pinned by pages/temp-list-deleted-row/Y66TempListGoneRow.al:235-460 (corpus pin 34ca226f), but this runtime refuses all 8 of its tests: each runs a second action on the list after it lost its only row, which is not built. Retire once they run.",
}
