PartBothKeepsRowAfterUpdate => {
    id: "part-both-keeps-row-after-update",
    assumed: "A ListPart with UpdatePropagation Both that stands on a stored row other than the first of its view and runs `CurrPage.Update(false)` in a control's OnValidate stays on that row on a Document, Card and ListPlus host, with or without a change of the host's stored row before: the host's triggers run first, then the part reads its rows as without Both (`R2 R3 R2 R1 R2 R2 C2`), the typed value is dropped, and the next SetValue is stored on the same row.",
    tests: "No Microsoft test. A partner app reported it (al-runner issue 6877: 14 tests that pass on a BC 28.5 container). Measured on BC 28.5 (Windows) by the probe below, 18 combinations.",
    probe: "R6877PartBothLaterRow.al on branch al-runtime-rs-staging-r6877: R_Doc_, R_Card_ and R_Plus_Nav1..3_Ctl1..2.",
    awaiting: Awaiting {
        issues: &[6877],
        files: &["tests/al-language/pages/lt-th-part-update-cursor/R6877PartBothLaterRow.al"],
        since: "2026-11-03",
    },
}
