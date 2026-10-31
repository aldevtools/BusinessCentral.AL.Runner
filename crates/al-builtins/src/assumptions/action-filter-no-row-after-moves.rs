ActionFilterNoRowAfterMoves => {
    id: "action-filter-no-row-after-moves",
    assumed: "After an action changes the filters of a list without row triggers so that the row it stood on leaves the view and no row of the new view follows it, the page stands on the nearest row before it and moves go on from there; a view left without rows shows a blank row on which First and Last answer false. (A row of the new view that follows is pinned by pages/lt-y5/Y5GoToKeyAfterActionFilter.al:158-174.)",
    tests: "No Microsoft test: al-runner issue 6468, the reporter's run on real BC (rows 1 and 2 included, row 3 not; on row 3 the action filters Included = true; First, Next, Next give 1, 2, false). Measured by corpus writer Y15 (Y15ActionNarrowsBelowLast.al, on the SaaS sandbox, newest major).",
    probe: "Y15ActionNarrowsBelowLast.al on branch al-runtime-rs-staging-y15: the row after the action, Next, Previous, GoToRecord, GoToKey, and an action that leaves no row.",
    awaiting: Awaiting {
        issues: &[6468],
        files: &["tests/al-language/pages/lt-y15/Y15ActionNarrowsBelowLast.al"],
        since: "2026-10-31",
    },
}
