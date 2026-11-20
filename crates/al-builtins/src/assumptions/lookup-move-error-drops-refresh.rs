LookupMoveErrorDropsRefresh => {
    id: "lookup-move-error-drops-refresh",
    assumed: "A host Next from the first row of the view, or a host First from its last row, that is the first call after a TestField.Lookup left the refresh of the OnValidate's CurrPage.Update(false) for later, and whose own OnAfterGetCurrRecord raises an error that closes the TestPage, drops that refresh: the same variable opens again as a fresh page and its next move reads once. Measured on every Business Central major the probe runs on, on the Y15LookupThreeFieldLink.al host shape. A move error that leaves the page open keeps the refresh as before. Refused when an error closes the page in a move that carries the refresh in any other shape: Previous, Last, Next from another row, First from another row, a later call after the Lookup, or an error raised elsewhere than OnAfterGetCurrRecord.",
    tests: "No Microsoft test: al-runner issue 8503.",
    probe: "Corpus pull request 1467 (merged, not pinned yet), tests/al-language/testpage/LookupLeftMoveStops.al:368 (Next_MoveReadFails), :450 (First_MoveReadFails).",
    awaiting: Awaiting {
        issues: &[8503],
        files: &["tests/al-language/testpage/LookupLeftMoveStops.al"],
        since: "2026-11-20",
    },
}
