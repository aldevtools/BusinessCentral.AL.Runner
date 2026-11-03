PageOpenTailThreeOrMoreListParts => {
    id: "page-open-tail-three-or-more-list-parts",
    assumed: "A host with function controls and three or four ListParts in its content area reads its current row after the parts opened, once per ListPart, then each ListPart but the last of the layout once less than the one before (host three times, first part twice, second part once with three parts), as with one or two ListParts. Measured on a Windows BC container by corpus writer R6806 (the `open log` of R6806_*_LookupReadEachControl); not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6806 (the order of trigger runs on a host with three or four parts).",
    probe: "R6806LookupParts.al R6806_*_LookupReadEachControl `open log` on corpus branch al-runtime-rs-staging-r6806.",
    awaiting: Awaiting {
        issues: &[6806],
        files: &[
            "tests/al-language/pages/lt-r6806-lookup-parts/R6806LookupParts.al::R6806_ABL_LookupReadEachControl",
        ],
        since: "2026-11-02",
    },
}
