LookupMoveErrorDropsRefresh => {
    id: "lookup-move-error-drops-refresh",
    assumed: "A host Next or First that is the first call after a TestField.Lookup left the refresh of the OnValidate's CurrPage.Update(false) for later, and that raises an error which closes the TestPage (measured: OnAfterGetCurrRecord raising in the move's own read, Next from the first row and First from the last row), drops that refresh: the same variable opens again as a fresh page and its next move reads once. Only when the error closed the page; Previous and Last, and a move error that leaves the page open, keep the refresh as before. Measured on every Business Central major the probe runs on, on the Y15LookupThreeFieldLink.al host shape.",
    tests: "No Microsoft test: al-runner issue 8503.",
    probe: "Corpus pull request 1467 (merged, not pinned yet), tests/al-language/testpage/LookupLeftMoveStops.al:368 (Next_MoveReadFails), :450 (First_MoveReadFails).",
    awaiting: Awaiting {
        issues: &[8503],
        files: &["tests/al-language/testpage/LookupLeftMoveStops.al"],
        since: "2026-11-20",
    },
}
