ReportSaveTopLevelItemsNoRow => {
    id: "report-save-top-level-items-no-row",
    assumed: "Report.SaveAs(Id, '', Format, OutStream) by number, with no request parameters and no RecordRef, to a stream, of a report whose dataset is exactly two or exactly three data items that are all top-level, whose run wrote no dataset row: the report declares its default layout with DefaultRenderingLayout and a layout file, no report trigger (OnInitReport, OnPreReport, OnPostReport, OnPreRendering), no report extension, is not processing only, and no Report Layout Selection or Tenant Report Layout Selection row of any company names the report or selects a layout. (1) A Word default layout, the answer used, as Pdf, Word or Html: false, nothing written, the error ReportResultEmpty ('The report couldn\u{2019}t be generated, because it was empty. Adjust your filters and try again.') for GetLastErrorText and GetLastErrorCode. (2) The same report with two data items, as Pdf, as a statement: that error is raised. (3) An RDLC default layout, two data items, the answer used, as Pdf or Word: true, nothing written. Not checked: the triggers of the data items and their table views (they run as in a save with rows; the probe measured items with no trigger over empty tables and items that stop in OnPreDataItem with CurrReport.Break). Every other save with no dataset row of such a report refuses: a file (SaveAsPdf and the other file forms, measured on one BC version only), a statement other than (2), Excel, a report trigger, a report extension, a layout selection, four or more items, nested items, request parameters, a RecordRef.",
    tests: "al-runner issue 8668: the reporter's repro saves a Word layout report with two top-level data items that each stop in OnPreDataItem as Pdf, as a statement; BC raises the empty-report error at Report.SaveAs.",
    probe: "Corpus pull request 1508 (branch probe-8668-saveas-empty-two-items, commit 7018f308, open), measured the same on the three supported BC majors: reports/saveas-empty-items/SaeTwoItemsTests.al:18 Word_EE, SaeThreeItemsTests.al:18 Word_EEE, SaeBreakTests.al:18 Word_BreakBoth, SaeRdlcTwoTests.al:18 Rdlc_EE (each with the answer used). Not measured, each needs a probe: the statement form of Word, Html and of three items, Excel, a report trigger, a report extension, a layout selection, nested items, request parameters; the file forms on the other majors (al-language-onprem/reports/layouts-saveas/SaeFileTests.al, measured on one major only).",
    awaiting: Awaiting {
        issues: &[8668],
        files: &[
            "tests/al-language/reports/saveas-empty-items/SaeTwoItemsTests.al",
            "tests/al-language/reports/saveas-empty-items/SaeThreeItemsTests.al",
            "tests/al-language/reports/saveas-empty-items/SaeBreakTests.al",
            "tests/al-language/reports/saveas-empty-items/SaeRdlcTwoTests.al",
        ],
        since: "2026-11-22",
    },
}
