ReportRunWithoutOutput => {
    id: "report-run-without-output",
    assumed: "Report.Run, RunModal and Execute of a report whose default layout is RDLC or Word run the report as the Xml path does (request page, triggers, data items) and write no output; the call answers as BC does (owner decision 2026-11-11, ADR-040 amendment). BC renders the layout and raises SelectReportLayoutCode, HasCustomLayout, SelectedBuiltinLayoutType, GetPrinterName, GetPaperTrayForReport, ApplicationReportMergeStrategy, SetupPrinters, OnDocumentReady and OnDocumentDownload there (events/platform-triggers/PeVTests.al:740-763, Windows values); this path raises none of them, and a subscriber the test can see refuses the run.",
    tests: "reports/runtime/LtTkShowPrintStatus.al:960-984 Layout_T_Run, _RunModal, _Execute and the Layout_F ones (an RDLC report: the triggers and rows run, the call answers Yes).",
    probe: "A corpus probe on the Windows nightly: Run, RunModal and Execute of a Word layout report with a subscriber logging the layout events, and the order of the report triggers against them.",
}
