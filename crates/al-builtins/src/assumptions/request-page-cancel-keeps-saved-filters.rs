RequestPageCancelKeepsSavedFilters => {
    id: "request-page-cancel-keeps-saved-filters",
    assumed: "A SaveValues request page closed by Cancel keeps the data item filters an earlier OK stored: a filter the handler changed before the Cancel is not stored, as a changed variable is not (pinned for variables by handlers/TestReportRequestPageSaveValues.al and misc/lt-probes-needed-6/PnF02SaveValuesExt.al; that an OK stores a data item filter is pinned by report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesFilter.al:P147_Filter_SetTableViewVsStored).",
    tests: "No Microsoft test: al-runner issue 6065, the reporter's run on real BC (a data item filter confirmed by OK, changed and cancelled on the next request page, read on the third).",
    probe: "A report with SaveValues and RequestFilterFields: OK with a filter A* set by the handler; next run SetFilter B* and Cancel; third run read the filter on the request page and in OnPreReport. Also with a filter cleared before the Cancel, and for a variable control next to it. Corpus writer: not yet assigned.",
    awaiting: Awaiting {
        issues: &[6065],
        files: &["tests/al-language/report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesFilterCancel.al"],
        since: "2026-10-28",
    },
}
