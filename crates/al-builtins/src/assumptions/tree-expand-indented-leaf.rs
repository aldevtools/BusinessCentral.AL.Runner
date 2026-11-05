TreeExpandIndentedLeaf => {
    id: "tree-expand-indented-leaf",
    assumed: "Expand(true) on a tree row without children under a parent, at any indentation, on a page with OnAfterGetRecord and OnAfterGetCurrRecord changes nothing: no error, IsExpanded stays No, no row trigger runs, the cursor and the rows shown stay.",
    tests: "No Microsoft test: al-runner issue 6947 (a partner app's test, the reporter's repro passed on real Business Central).",
    probe: "pages/tree-expand-leaf/TreeExpandIndentedLeaf.al on corpus branch al-runtime-rs-staging-r6945, measured on the corpus CI legs of all three supported majors.",
    awaiting: Awaiting {
        issues: &[6947],
        files: &["tests/al-language/pages/tree-expand-leaf/TreeExpandIndentedLeaf.al"],
        since: "2026-11-04",
    },
}
