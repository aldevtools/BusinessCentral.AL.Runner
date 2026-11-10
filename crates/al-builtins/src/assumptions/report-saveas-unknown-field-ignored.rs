ReportSaveAsUnknownFieldIgnored => {
    id: "report-saveas-unknown-field-ignored",
    assumed: "Report.SaveAs with a parameters string whose Options hold a Field that names no variable a control of the request page shows ignores that Field and applies the others. Pinned and not part of the assumption: Report.Execute skips such a Field (reports/WdReportExecuteParams.al:69-77 WD_ReportExecute_UnknownField). A data item name the request page does not have stays refused under SaveAs, and an unknown Field under RunRequestPage stays refused.",
    tests: "No Microsoft test. A user's report: an unknown Field next to a known one in the parameters of Report.SaveAs with ReportFormat::Xml leaves the known option set and the other option at its OnInitReport value on BC 29.0 (al-runner issue 7590).",
    probe: "reports/request-page/ReportParametersBareRoot.al: Report.SaveAs (Xml) with an unknown Field next to a known one, the root with and without name and id; the options the report reads in OnPreReport.",
    awaiting: Awaiting {
        issues: &[7590],
        files: &["tests/al-language/reports/request-page/ReportParametersBareRoot.al"],
        since: "2026-11-10",
    },
}
