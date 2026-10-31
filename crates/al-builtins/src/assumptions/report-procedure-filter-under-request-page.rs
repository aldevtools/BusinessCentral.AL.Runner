ReportProcedureFilterUnderRequestPage => {
    id: "report-procedure-filter-under-request-page",
    assumed: "A filter a procedure of the report set on a data item before a run with a request page is not in that run: the page does not show it (pinned) and the filters of the page replace it when the page closes with OK (assumed).",
    tests: "Microsoft tests that call a report procedure which filters a data item and then RunModal the report with its request page.",
    probe: "A report procedure RowDi.SetFilter(Code, 'B*'), RunModal with a handler that clicks OK and a report that counts the rows in OnPreDataItem and OnAfterGetRecord: are the B rows alone read? Asked in corpus #646.",
}
