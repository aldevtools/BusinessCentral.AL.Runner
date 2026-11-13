ReportDatasetColumnLineBreaks => {
    id: "report-dataset-column-line-breaks",
    assumed: "Static Report.SaveAs to an XML OutStream normalizes CR, LF and CRLF in Text dataset columns to CRLF.",
    tests: "al-runner issue 7905 and corpus PR #1359, reports/dataset/TestReportDatasetLineBreaks.al (BC 27.5, 28.5 and 29.0).",
    probe: "Corpus PR #1359, reports/dataset/TestReportDatasetLineBreaks.al.",
    awaiting: Awaiting {
        issues: &[7905],
        files: &["tests/al-language/reports/dataset/TestReportDatasetLineBreaks.al"],
        since: "2026-11-13",
    },
}
