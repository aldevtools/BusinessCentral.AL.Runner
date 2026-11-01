RequestPageCaptionSet => {
    id: "request-page-caption-set",
    assumed: "`RequestOptionsPage.Caption := Text` in the code of a report outside the request page's OnOpenPage (in OnInit, a control's OnValidate, a report trigger, a report extension) does not change the caption a request page handler reads (TestRequestPage.Caption). Set in OnOpenPage it is the handler's caption (pinned by report/lt-aq/AqTests.al:313-330); the report's code reads a caption set anywhere back (pinned by report/lt-y40-bare-report-run/Y40BareReportRun.al:344, corpus master ac3499d6).",
    tests: "Financial Report (`RequestOptionsPage.Caption := FinancialReport.Description` in ValidateAccSchedName, also called from OnValidate of the request page controls): about 86 Tests-ERM tests.",
    probe: "A report whose request page sets RequestOptionsPage.Caption in OnInit and in a control's OnValidate; the handler then reads TestRequestPage.Caption (after SetValue).",
}
