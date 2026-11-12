ReportHtmlNoDatasetRow => {
    id: "report-html-no-dataset-row",
    assumed: "Report.SaveAs(.., ReportFormat::Html, OutStream) with its answer used, of a report with a Word default layout, one data item and no report extension, whose run wrote no dataset row, answers false, raises no error and writes nothing to the stream (ADR-040). Every other save with no dataset row of a Word layout report refuses (another format, a file, the statement form, more data items, a temporary layout selection).",
    tests: "Corpus probe pull request 1362 (branch al-runtime-rs-staging-opaque-text, commit 928b6a93), reports/layouts-html-text/HtRenderedTextTests.al:320-335 Html_SaveAs_NoDatasetRow: `saved` No, `length > 0` No, `ok` Yes, measured on the two older BC majors (the newest not measured).",
    probe: "reports/layouts-html-text/HtRenderedTextTests.al:320-335 Html_SaveAs_NoDatasetRow (corpus pull request 1362, open). It pins the cell once the pull request is merged and the corpus is bumped; the newest BC major still to be measured.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/reports/layouts-html-text/HtRenderedTextTests.al"],
        since: "2026-11-12",
    },
}
