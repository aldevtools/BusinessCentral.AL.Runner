RequestPageCaptionSet => {
    id: "request-page-caption-set",
    assumed: "`RequestOptionsPage.Caption := Text` in the code of a report outside the request page's OnOpenPage (in OnInit, a control's OnValidate, a report trigger, a report extension) changes nothing a test can see. Set in OnOpenPage it is the handler's caption (pinned by report/lt-aq/AqTests.al:313-330).",
    tests: "Financial Report (`RequestOptionsPage.Caption := FinancialReport.Description` in ValidateAccSchedName, also called from OnValidate of the request page controls): about 86 Tests-ERM tests.",
    probe: "A report whose request page sets RequestOptionsPage.Caption in OnInit, in a control's OnValidate (the handler then reads TestRequestPage.Caption after SetValue), and in OnPreReport.",
}
