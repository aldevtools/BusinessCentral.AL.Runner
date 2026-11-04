PartReadOnlyNextAtEnd => {
    id: "part-read-only-next-at-end",
    assumed: "Next() from the last row of a part whose part control says Editable = false answers No and the part stays on that row (no new-row line), as on a part page that is not editable or allows no insert.",
    tests: "No Microsoft test: al-runner issue 6926 (a partner app's test walks a non-editable part with First and Next). Observed on the newest major by the corpus probe below.",
    probe: "R6926PartNextReadOnly.al R6926_* tests (codeunit 95173) on corpus branch al-runtime-rs-staging-r6926, corpus PR 1116.",
    awaiting: Awaiting {
        issues: &[6926],
        files: &["tests/al-language/pages/parts/R6926PartNextReadOnly.al"],
        since: "2026-11-04",
    },
}
