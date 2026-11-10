ReportSaveAsUnknownFieldIgnored => {
    id: "report-saveas-unknown-field-ignored",
    assumed: "Report.SaveAs with a parameters string whose root has no name and id (the XML declaration, <ReportParameters> holding one <Options>) ignores a Field whose name the report has nowhere (no control, no global of the report or of an extension of its request page, in any letter case) and applies the others. Refused, not part of the assumption: the same Field under a root with name and id, a Field that names a global no control shows, a Field that differs from a shown variable only in letter case, a Field named by a control. Pinned and not part of the assumption: Report.Execute skips an unknown Field (reports/WdReportExecuteParams.al:69-77 WD_ReportExecute_UnknownField).",
    tests: "No Microsoft test. A user's report: an unknown Field next to a known one in the parameters of Report.SaveAs with ReportFormat::Xml leaves the known option set and the other option at its OnInitReport value on real BC (al-runner issue 7590).",
    probe: "reports/request-page/ReportParametersBareRoot.al: Report.SaveAs (Xml) with an unknown Field next to a known one, the root with and without name and id; a Field that names a global no control shows, one that differs from a shown variable only in letter case, one named by a control; the options the report reads in OnPreReport.",
    awaiting: Awaiting {
        issues: &[7590],
        files: &["tests/al-language/reports/request-page/ReportParametersBareRoot.al"],
        since: "2026-11-10",
    },
}
