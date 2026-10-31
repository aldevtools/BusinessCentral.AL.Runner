ReportViewOverStoredRequestFilters => {
    id: "report-view-over-stored-request-filters",
    assumed: "A data item the caller gave a view (SetTableView, or the record of Report.Run or RunModal) keeps that view when the report has SaveValues and an earlier run stored other filters for the data item. (That an OK stores the view's filters is pinned by report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesFilter.al:P147_Filter_SetTableViewVsStored.)",
    tests: "Tests-ERM tests that run `Suggest Fin. Charge Memo Lines` for a second memo in one codeunit (IssueFinanceChargeMemoWith* and others).",
    probe: "A report with SaveValues: run it once with the filter A* stored by OK, then with SetTableView(C*) and see the data item's filter on the request page and in the run; the same with the record of Report.RunModal. Asked in corpus #646.",
}
