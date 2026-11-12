ExcelLayoutMultipleDataSheets => {
    id: "excel-layout-multiple-data-sheets",
    assumed: "A report that declares ExcelLayoutMultipleDataSheets = true runs as the same report without it in the measured shapes: Report.SaveAs as Xml (no layout, a Word or an Excel default layout) writes the same dataset; Report.Run and RunModal without the request page of the report with no layout or an Excel default layout raise ReportNoLayout or ReportUnsupportedOperation; Report.SaveAs as Excel of the report with an Excel default layout writes a placeholder (not empty) and with no layout writes nothing. Every other run of such a report refuses.",
    tests: "Corpus probe branch al-runtime-rs-writer-reports6 (pull request not opened yet), reports/layouts-saveas/ExcelMultipleDataSheetsReport.al, observed on the three majors the corpus runs.",
    probe: "Corpus probe branch al-runtime-rs-writer-reports6 (pull request not opened yet), reports/layouts-saveas/ExcelMultipleDataSheetsReport.al: SaveAs Xml compared with the report without the property, Run and RunModal without the request page, SaveAs Excel.",
    awaiting: Awaiting {
        issues: &[7836],
        files: &["tests/al-language/reports/layouts-saveas/ExcelMultipleDataSheetsReport.al"],
        since: "2026-11-12",
    },
}
