RequestPageCaptionSet => {
    id: "request-page-caption-set",
    assumed: "`RequestOptionsPage.Caption := Text` in the code of a report in a report trigger or a report extension (not in the request page's own OnInit or OnOpenPage) does not change the caption a request page handler reads (TestRequestPage.Caption). Pinned and not assumed: a set in the request page's OnInit is the handler's caption, OnOpenPage wins over it, and a set in a control's OnValidate changes what the report's code reads back but not what the handler reads (reports/request-page/Y86RequestCaption.al); a set in OnOpenPage (reports/AqTests.al:313-330); the report's code reads a caption set anywhere back (reports/runtime/Y40BareReportRun.al:344).",
    tests: "Financial Report (`RequestOptionsPage.Caption := FinancialReport.Description` in ValidateAccSchedName, also called from OnValidate of the request page controls): about 86 Tests-ERM tests.",
    probe: "A report whose OnPreReport, OnInitReport or reportextension trigger sets RequestOptionsPage.Caption before the request page opens; the handler then reads TestRequestPage.Caption.",
}
