RepeaterEditableDecidesNewRowLine => {
    id: "repeater-editable-decides-new-row-line",
    assumed: "The value of a repeater's Editable decides whether a list shows the blank new-row line: false (the literal, or a page variable that is false) shows none on a List, a Worksheet and a ListPart, so Next on the last row answers false and stays there; true shows it. New() raises nothing on such a page.",
    tests: "No Microsoft test: al-runner issue 6674, the reporter's Page.Run with a PageHandler on a List whose repeater says Editable = false.",
    probe: "Measured on a real SaaS sandbox by corpus writer Y65 (branch al-runtime-rs-staging-y65 eefaf01b, corpus PR to master open).",
    awaiting: Awaiting {
        issues: &[6674],
        files: &["tests/al-language/pages/lt-y65-readonly-repeater/Y65ReadOnlyRepeater.al"],
        since: "2026-11-01",
    },
}
