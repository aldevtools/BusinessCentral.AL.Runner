ActionWidensViewHoldsAll => {
    id: "action-widens-view-holds-all",
    assumed: "An action that changes the filters of a list with OnAfterGetRecord so that rows of the new view lie before the row it stood on, the first row of the old view: the refresh reads as on a later row (the way back to the first row), so First lands on the first row of the new view without reading.",
    tests: "No Microsoft test: al-runner issue 6470, the reporter's run on real BC (25 bins, the page on A20, ShowAll: First gives A01, Next walks 25 rows). Measured by corpus writer Y15 (Y15ActionWidensView.al, on the SaaS sandbox, newest major).",
    probe: "Y15ActionWidensView.al on branch al-runtime-rs-staging-y15: the reads of the action, First, Next to the end, Last.",
    awaiting: Awaiting {
        issues: &[6470],
        files: &["tests/al-language/pages/lt-y15/Y15ActionWidensView.al"],
        since: "2026-10-31",
    },
}
