ReportHtmlNoDatasetRow => {
    id: "report-html-no-dataset-row",
    assumed: "Report.SaveAs(.., ReportFormat::Html, OutStream) with its answer used, of a report with a Word default layout, one data item and no report extension, whose run wrote no dataset row, answers false, raises no error, writes nothing to the stream and leaves the error ReportResultEmpty ('The report couldn\u{2019}t be generated, because it was empty. Adjust your filters and try again.') for GetLastErrorText and GetLastErrorCode (ADR-040). Every other save with no dataset row of a Word layout report refuses (another format, a file, the statement form, more data items, a temporary layout selection).",
    tests: "Corpus probe pull request 1362 (branch al-runtime-rs-staging-opaque-text, commit 928b6a93), reports/layouts-html-text/HtRenderedTextTests.al:320-335 Html_SaveAs_NoDatasetRow: `saved` No, `length > 0` No, `ok` Yes, measured on the two older BC majors. Corpus probe pull request 1371 (branch al-runtime-rs-staging-reports-regex, commit 05c80012, green on the three BC majors the corpus runs), reports/html-email-body/RxEmailBodyTests.al:134-143 WordDefault_NoRow_EmailBody: `returned` No, the error text and the code ReportResultEmpty after a TryFunction that completed.",
    probe: "reports/layouts-html-text/HtRenderedTextTests.al:320-335 Html_SaveAs_NoDatasetRow (corpus pull request 1362, open) and reports/html-email-body/RxEmailBodyTests.al:134-143 WordDefault_NoRow_EmailBody (corpus pull request 1371, open). They pin the cell once the pull requests are merged and the corpus is bumped. Not measured: a Pdf, Word or Excel save with no dataset row, a save to a file, the statement form, more than one data item.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &[
            "tests/al-language/reports/layouts-html-text/HtRenderedTextTests.al",
            "tests/al-language/reports/html-email-body/RxEmailBodyTests.al",
        ],
        since: "2026-11-12",
    },
}
