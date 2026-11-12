ReportLayoutTempSelection => {
    id: "report-layout-temp-selection",
    assumed: "Report.SaveAs(.., ReportFormat::Html, OutStream) of a report with a Word default layout and no report extension, while Base Application's codeunit \"Design-time Report Selection\" holds a layout name with no app id, renders the Tenant Report Layout row of that report with exactly that name, a blank Company Name, the null App ID and Layout Format Word (ADR-040 amendment 2026-11-12, A7). The runtime raises SelectReportLayoutCode and then SelectedBuiltinLayoutType (the order of events/platform-triggers/PeVTests.al:721) to Base Application's ReportManagement subscribers only, and goes on only when the first answers Success with that name and layout type None and the second answers None, no Tenant Report Layout Selection row names the report, and no other Tenant Report Layout row of the report has the name in another case. Every other selection refuses.",
    tests: "al-runner issue 6924: the reporter's e-mail body tests select a user-defined Word layout (a Tenant Report Layout row named by a GUID) through Report Selections.SaveReportAsHTML, whose SaveReportAsHTMLImpl calls Report Layout Selection.SetTempLayoutSelectedName before the save (they pass on BC).",
    probe: "reports/layouts-html-text/HtRenderedTextTests.al TempLayout_* (corpus branch al-runtime-rs-staging-opaque-text, written in observe form, pull request not opened yet). Windows tier: insert a Tenant Report Layout row of Layout Format Word named by a GUID for a Word layout report, Report Layout Selection.SetTempLayoutSelectedName(name, null guid), Report.SaveAs as Html to a Temp Blob: the answer, a non-empty output, the selection cleared after the save, and a logging subscriber to SubstituteReport, SelectReportLayoutCode, HasCustomLayout, SelectedBuiltinLayoutType and OnDocumentReady (the events, their values and order); the same with a name that matches no layout (probe cell S5 of ADR-040 B5) and with an RDLC row.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/reports/layouts-html-text/HtRenderedTextTests.al"],
        since: "2026-11-12",
    },
}
