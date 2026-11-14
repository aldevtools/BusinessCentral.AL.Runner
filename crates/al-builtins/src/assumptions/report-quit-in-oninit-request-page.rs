ReportQuitInOninitRequestPage => {
    id: "report-quit-in-oninit-request-page",
    assumed: "CurrReport.Quit directly in OnInitReport of a report with ProcessingOnly = true, a request page without an OnInit trigger, no UseRequestPage property and no report extension (al-vm reports/quit_in_init.rs): RunRequestPage on the variable that first uses the report, and the static RunRequestPage with or without a parameters string, answer '' and open no page; an Xml SaveAs to a stream with an empty parameters string and its answer used, on a variable or static, answers false and writes nothing (GetLastErrorText after it is not measured and left as it was); UseRequestPage(false) as the first use of a variable keeps the quit, and the next Run or RunModal of that variable runs nothing (a second Run runs OnInitReport again); a Clear of that variable gives it a new instance without the kept quit, as Clear does on main, so its next Run runs OnInitReport again. Any other Report method or procedure call on that variable between the kept quit and its Run or RunModal, a SaveAs with its answer dropped, and a report outside that shape keep the refusal. A Run, RunModal or Execute that runs OnInitReport itself is main's path and not flagged.",
    tests: "No Microsoft test. Outside report: al-runner issue 7964 item 3.",
    probe: "Corpus PR 1382, merged on corpus master as 23337cd2, not pinned yet (reports/runtime/ReportQuitInOnInit.al:235-355), the same on every BC major of the probe.",
    awaiting: Awaiting {
        issues: &[7964],
        files: &["tests/al-language/reports/runtime/ReportQuitInOnInit.al"],
        since: "2026-11-13",
    },
}
