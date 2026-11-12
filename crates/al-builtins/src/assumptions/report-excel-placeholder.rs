ReportExcelPlaceholder => {
    id: "report-excel-placeholder",
    assumed: "Report.SaveAs to Excel, Report.SaveAsExcel and TestRequestPage.SaveAsExcel of a report whose default layout is Excel run the report as the Xml path does, answer true and write a placeholder .xlsx (a zip with xl/workbook.xml and one empty sheet) instead of BC's rendering (owner decision 2026-11-11, ADR-040 amendment). Only 'it exists, it is a zip, it is not empty' is meant to match BC; Excel cell checks read the content and are refused. The layout events BC raises around the render are not raised, and a subscriber the test can see refuses the save.",
    tests: "reports/layouts-saveas/AMReportOutputTests.al:235 ExcelLayout_SaveAsExcel and reports/CiReportRunLeads.al:262-266 ExcelOnly (true, length > 0) for the static stream form on the Linux tier.",
    probe: "A corpus probe on the Windows nightly: SaveAsExcel(FileName) of an Excel layout report, then File.Exists and the size; the variable form; TestRequestPage.SaveAsExcel.",
}
