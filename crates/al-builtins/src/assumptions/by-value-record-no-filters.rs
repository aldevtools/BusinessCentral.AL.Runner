ByValueRecordNoFilters => {
    id: "by-value-record-no-filters",
    assumed: "A record of a stored table handed by value to a Record parameter of a procedure (not declared temporary, not an event subscriber's), whose filters are all in filter group 0 and whose current filter group is 0, with the primary key ascending as the copy gets it (pinned: record/keys-sorting/PwFlowFieldKeyRule.al:252), no marks and MarkedOnly off, gives the callee no filters: GetFilters reads '' and FindFirst finds a row outside the caller's filter. Filters in another group, a descending order or marks refuse every read of the copy's view (al-vm records/by_value_view.rs `BY_VALUE_VIEW`).",
    tests: "al-runner issue 8185: the reporter's repro measured on a BC container (`by value: {}`, `FindFirst by value: Yes`); the issue has the code.",
    probe: "A stored record with a filter in group 0 handed by value: GetFilters, Count, FindFirst, GetView in the callee; the same with filters in groups 1, 2, 4 and -1, the caller standing in another filter group, Ascending(false), marks with and without MarkedOnly(true), SetLoadFields, ChangeCompany; and the caller's filters after the call.",
    awaiting: Awaiting {
        issues: &[8185],
        files: &["tests/al-language/record/filters/ByValueRecordView.al"],
        since: "2026-11-17",
    },
}
