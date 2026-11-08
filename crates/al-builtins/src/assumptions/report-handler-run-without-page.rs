ReportHandlerRunWithoutPage => {
    id: "report-handler-run-without-page",
    assumed: "A RunModal without the request page of a report that renders output (not processing-only) calls the test's [ReportHandler] bound for it, and the run ends there, as a Run does (reports/layouts-saveas/Y43SystemPrinterRequestPage.al:313, on master). Pinned on real BC by corpus pull request 1199 (reports/reporthandler/AeReportHandler.al:237, not yet on master): the handler runs and nothing else is logged. A processing-only report run without its page is left as it was.",
    tests: "No Microsoft test: al-runner issue 7269 (the reporter's real-BC run of Report_RunModal_RdlcReportCallsRegisteredHandler on the three supported majors, corpus pull request 1199).",
    probe: "reports/reporthandler/AeReportHandler.al once merged; also wanted: a local rendering report with OnInitReport and a request page OnInit that log, run without the page under a [ReportHandler], and a processing-only report run without its page under one.",
    awaiting: Awaiting {
        issues: &[7269],
        files: &["tests/al-language/reports/reporthandler/AeReportHandler.al::Report_RunModal_RdlcReportCallsRegisteredHandler"],
        since: "2026-11-08",
    },
}
