ActionEmptyView => {
    id: "action-empty-view",
    assumed: "An action of a list with OnAfterGetRecord and OnAfterGetCurrRecord whose filter leaves the view without rows and that calls CurrPage.Update shows a blank row (repeater controls blank, page variables too) and runs no OnAfterGetCurrRecord; after the update OnAfterGetRecord runs for the row before the current one of the old view (the row after it when there is none) or for every other row, by major (`al_version::empty_action_view_reads_every_row`); moves and GoTo answer false; an action on the blank row with Update(true) or Update(false) that brings rows back fills the page as at open.",
    tests: "No Microsoft test: al-runner issue 6945 (a partner app's test, the reporter's repro passed on real Business Central).",
    probe: "pages/actions/ActionFilterEmptyView.al on corpus branch al-runtime-rs-staging-r6945, measured on the corpus CI legs of all three supported majors.",
    awaiting: Awaiting {
        issues: &[6945],
        files: &["tests/al-language/pages/actions/ActionFilterEmptyView.al"],
        since: "2026-11-04",
    },
}
