ReportQuitInOninitRequestPage => {
    id: "report-quit-in-oninit-request-page",
    assumed: "PLACEHOLDER",
    tests: "No Microsoft test. Outside report: al-runner issue 7964 item 3.",
    probe: "Corpus PR 1382, merged on corpus master as 23337cd2, not pinned yet (reports/runtime/ReportQuitInOnInit.al:235-355), the same on BC 27.5, 28.5 and 29.0.",
    awaiting: Awaiting {
        issues: &[7964],
        files: &["tests/al-language/reports/runtime/ReportQuitInOnInit.al"],
        since: "2026-11-13",
    },
}
