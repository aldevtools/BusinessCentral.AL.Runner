ReportWordPlaceholder => {
    id: "report-word-placeholder",
    assumed: "Report.SaveAs to Word and Report.SaveAsWord of a report whose default layout is Word run the report as the Xml path does, answer true and write a placeholder .docx (a zip whose main part is word/document.xml with one empty paragraph) instead of BC's rendering (owner decision 2026-11-11, ADR-040 amendment). Only 'it exists, it is a zip, it is not empty' is meant to match BC; a read of the content is refused. The layout events BC raises around the render are not raised, and a subscriber the test can see refuses the save. The file form (SaveAsWord(FileName)) and the variable form are not measured.",
    tests: "reports/layouts-saveas/AMReportOutputTests.al:187 WordLayout_SaveAsWord and reports/CiReportRunLeads.al:596-600 TargetFormat_PageNo_WordDefaultLayout pin true and a non-empty zip for the static stream form on the Linux tier.",
    probe: "A corpus probe on the Windows nightly: SaveAsWord(FileName) of a Word layout report, then File.Exists and the size; the variable form; Run/RunModal/Execute with a logging subscriber.",
}
