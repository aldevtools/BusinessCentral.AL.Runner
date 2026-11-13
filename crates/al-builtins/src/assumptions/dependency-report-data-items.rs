DependencyReportDataItems => {
    id: "dependency-report-data-items",
    assumed: "Static Report.SaveAs to an XML stream with empty parameters and no RecordRef applies the measured pair of report extensions in dependency order, for either object ID order. A direct consumer can modify and add columns on the owner's item under a base item and place siblings next to its nested item, including a link to the immediate parent.",
    tests: "al-runner issue 7878 and corpus PR #1342, reports/extensions/TestReportDependencyDataItems.al, on BC 27.5, 28.5 and 29.0.",
    probe: "Corpus PR #1342 measures the reversed and rising extension ID orders.",
    awaiting: Awaiting {
        issues: &[7878],
        files: &["tests/al-language/reports/extensions/TestReportDependencyDataItems.al"],
        since: "2026-11-13",
    },
}
