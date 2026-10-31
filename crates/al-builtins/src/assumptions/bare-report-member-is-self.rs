BareReportMemberIsSelf => {
    id: "bare-report-member-is-self",
    assumed: "In a procedure of a report, the bare `UseRequestPage(..)` and `RunModal()` (without `CurrReport.` or `this.`) work on the report's own instance, the one whose procedure runs, as `Rep.UseRequestPage(false); Rep.RunModal()` on a report variable does (the receiver form is pinned by codeunit/TestWriteTxTestBoundary.al:272). The bare form in a trigger, while the report runs, is refused.",
    tests: "No BCApps test: a public app's test (a processing-only report whose procedure sets UseRequestPage(false) and runs itself modally to look for records; 1 test).",
    probe: "A report with a procedure that calls the bare UseRequestPage(false) and RunModal(): run from a report variable, the request page does not open (no handler), the report's triggers run once, its globals set by the triggers read back through the variable afterwards, and a second call of the procedure gives the RunModal error of a variable already run.",
}
