BarePageRunIsStatic => {
    id: "bare-page-run-is-static",
    assumed: "A bare `Run(Page::X[, Rec[, FieldNo]])` in a page's code is the static `Page.Run`: the page opens with the record's filters and the action continues, as the bare `RunModal(Page::X, Rec)` is the static `Page.RunModal` (pages/lt-uf-unqualified-runmodal/UfUnqualifiedRunModal.al:71-117). Measured on real BC by corpus writer Y47.",
    tests: "No Microsoft test: al-runner issue 6590 (38 tests of the reporter's app).",
    probe: "A bare `Run(Page::X, Rec)`, `Run(Page::X)`, `Run(Page::X, Rec, FieldNo)` and `Run(<id>, Rec)` in a page action: the page handler runs with the record's filters, the action goes on.",
    awaiting: Awaiting {
        issues: &[6590],
        files: &["tests/al-language/pages/lt-uf-unqualified-runmodal/Y47UnqualifiedRun.al"],
        since: "2026-11-01",
    },
}
