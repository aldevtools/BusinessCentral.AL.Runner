TreeCutTestPageOnly => {
    id: "tree-cut-testpage-only",
    assumed: "A tree page (ShowAsTree over an Integer IndentationColumn field) cuts its rows only for what the TestPage shows: the page's AL code sees every row of Rec (Count, FindSet, GetFilters; Rec.Reset() changes nothing), the cut is made after OnOpenPage, and the TestPage shows the rows without a parent in the view plus the children of the parent of the row Rec stands on after OnOpenPage or an action.",
    tests: "No Microsoft test: al-runner issue 6523, the reporter's run on real BC, two majors (Rec.Reset() in OnOpenPage walks Root 1, Root 2; Rec.Count() in OnOpenPage reads 5). Measured by corpus writer Y28 (Y28CollapsedTreeTests.al, on the SaaS sandbox, newest major).",
    probe: "Y28CollapsedTreeTests.al on branch al-runtime-rs-staging-y28: Count and filters in OnOpenPage, row triggers and actions; Reset, FindSet, Get of a child and SetRange in OnOpenPage; Reset and Get in an action.",
    awaiting: Awaiting {
        issues: &[6523],
        files: &["tests/al-language/pageext/lt-page-ext-properties/Y28CollapsedTreeTests.al"],
        since: "2026-10-31",
    },
}
