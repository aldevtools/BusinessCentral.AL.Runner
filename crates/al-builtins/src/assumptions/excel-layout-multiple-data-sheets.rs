ExcelLayoutMultipleDataSheets => {
    id: "excel-layout-multiple-data-sheets",
    assumed: "A report that declares ExcelLayoutMultipleDataSheets = true runs as the same report without it in the measured calls, each a static call on the report number: Report.SaveAs(Id, '', ReportFormat::Xml, OutStream) of the report with no layout, or one Word or Excel rendering layout that is the default, writes the same dataset; Report.SaveAs(Id, '', ReportFormat::Excel, OutStream) writes a placeholder (not empty) with the Excel layout and nothing with no layout; Report.Run(Id, false) and Report.RunModal(Id, false) of the report with no layout or the Excel layout raise ReportNoLayout or ReportUnsupportedOperation. The report measured declares UseRequestPage = false, no request page, no trigger or procedure, is not ProcessingOnly, has no RDLCLayout, WordLayout or ExcelLayout property and no report extension. The probe ran with SubstituteReport subscribers that leave NewReportId as it is (Base Application's ReportManagement, and codeunit 79254 of the corpus app, reports/layouts-saveas/AMReports.al:229-234): the call is taken only when the report that runs after SubstituteReport is the report asked for. Every other run of such a report, and every use of a report variable of it, refuses.",
    tests: "Corpus probe branch al-runtime-rs-writer-reports6 (pull request 1368), reports/layouts-saveas/ExcelMultipleDataSheetsReport.al, observed on the three majors the corpus runs.",
    probe: "Corpus probe branch al-runtime-rs-writer-reports6 (pull request 1368), reports/layouts-saveas/ExcelMultipleDataSheetsReport.al: SaveAs Xml compared with the report without the property, Run and RunModal without the request page, SaveAs Excel.",
    awaiting: Awaiting {
        issues: &[7836],
        files: &["tests/al-language/reports/layouts-saveas/ExcelMultipleDataSheetsReport.al"],
        since: "2026-11-18",
    },
}
