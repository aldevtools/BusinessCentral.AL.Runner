ReportExtensionExplicitParentLink => {
    id: "report-extension-explicit-parent-link",
    assumed: "A report extension data item with a DataItemLink can explicitly name its immediate parent in DataItemLinkReference, whether the parent is a base item or another added item.",
    tests: "al-runner issue 7904 and corpus PR #1358, reports/extensions/TestReportExtensionParentLink.al:72 (BC 27.5, 28.5 and 29.0).",
    probe: "Corpus PR #1358, reports/extensions/TestReportExtensionParentLink.al:47-61.",
    awaiting: Awaiting {
        issues: &[7904],
        files: &["tests/al-language/reports/extensions/TestReportExtensionParentLink.al"],
        since: "2026-11-13",
    },
}
