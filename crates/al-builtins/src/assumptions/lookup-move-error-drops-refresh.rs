LookupMoveErrorDropsRefresh => {
    id: "lookup-move-error-drops-refresh",
    assumed: "A host move (Next, or First from the last row) that is the first call after a TestField.Lookup left the refresh of the OnValidate's CurrPage.Update(false) for later, and whose own read raises an error in OnAfterGetCurrRecord, closes the TestPage and drops that refresh: the same variable opens again as a fresh page and its next move reads once. Only when the error closed the page; a move error that leaves the page open keeps the refresh as before. Measured on Business Central 27.5, 28.5 and 29.0 on the Y15LookupThreeFieldLink.al host shape.",
    tests: "No Microsoft test: al-runner issue 8503.",
    probe: "Corpus pull request 1467 (merged, not pinned yet), tests/al-language/testpage/LookupLeftMoveStops.al:368 (Next_MoveReadFails), :450 (First_MoveReadFails).",
    awaiting: Awaiting {
        issues: &[8503],
        files: &["tests/al-language/testpage/LookupLeftMoveStops.al"],
        since: "2026-11-20",
    },
}
