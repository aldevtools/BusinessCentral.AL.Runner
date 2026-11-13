ReportSaveDeclWordRdlc => {
    id: "report-save-decl-word-rdlc",
    assumed: "Report.SaveAs(.., ReportFormat::Pdf|Word|Excel|Html, OutStream) of a report that declares exactly a WordLayout and an RDLCLayout property with DefaultLayout = Word (no rendering section, no ExcelLayout), one data item, no report trigger and no report extension, saved to a stream (answer used or a statement) with dataset rows, while no subscriber the test can see listens to a layout event. (1) Base Application's ReportManagement answers SelectedBuiltinLayoutType with RDLC (a Report Layout Selection row of the company selects \"RDLC (built-in)\") and SelectReportLayoutCode with no layout: the RDLC layout renders Pdf, Word and Excel; Html answers false with ReportUnsupportedFormat and a statement raises it. (2) SelectReportLayoutCode answers the name of exactly one user-defined Word layout of the report (Tenant Report Layout, blank Company Name, null App ID) that \"Design-time Report Selection\" held, while SelectedBuiltinLayoutType answers None, RDLC or Word: that layout renders Html, Word and Pdf; Excel answers false with ReportUnsupportedFormat and a statement raises it; the rgba replace of Email Message.Create is measured on its Html (answer used). The rows of al-vm report_save/cells_decl.rs; every other shape keeps its row or refuses.",
    tests: "al-runner issue 6924: the reporter's suite saves such a report after a company RDLC (built-in) row and SetTempLayoutSelected(''), and through Report Selections.SaveReportAsHTML with a GUID-named user-defined Word layout (they pass on BC).",
    probe: "Corpus pull request 1381 (branch al-runtime-rs-staging-reports-decl, commit c3745a3d, open; observed on BC 27.5, 28.5 and 29.0 with the same values): reports/html-email-body/RxdDeclLayoutTests.al:130-:293 (company RDLC row with SetTempLayoutSelected('')), :706-:784, :1231, :1441 (alone, both rows), :313-:391 (no row, GUID selection), :811, :1336, :1651 (GUID selection over a stored row); RxdDeclEmailBodyTests.al:69, :199, :251, :329, :407 (rgba replace on the selected layout's Html).",
    awaiting: Awaiting {
        issues: &[6924],
        files: &[
            "tests/al-language/reports/html-email-body/RxdDeclLayoutTests.al",
            "tests/al-language/reports/html-email-body/RxdDeclEmailBodyTests.al",
        ],
        since: "2026-11-13",
    },
}
