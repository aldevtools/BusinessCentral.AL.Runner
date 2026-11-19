GetFiltersFlowfilterBeforeFlowfield => {
    id: "get-filters-flowfilter-before-flowfield",
    assumed: "Record.GetFilters of a record in filter group 0, not on MarkedOnly, whose filter group holds exactly two filters, one on a FlowField and one on a FlowFilter field, both declared by the table itself, the FlowField before the FlowFilter both by declaration position and by field number, the FlowFilter's set first (the last SetRange or SetFilter of each field counts), lists the FlowFilter first. Every other shape where the FlowField comes first by declaration position or by field number refuses: the filters set in the other order, more filters, another filter group, MarkedOnly, fields declared out of number order, a field of a table extension, RecordRef.GetFilters and the filter list of a Find error.",
    tests: "Corpus PR 1430 (merged 2026-11-18, merge commit 75d895e1, not in the pinned corpus yet): tests/al-language/record/filters/ByValueRecordView.al:569-616 (ByValue_FlowFieldFilter_Codeunit, ByValue_FlowFieldFilter_LocalProcedure, ByValue_TableProcedure_OwnRecByValue): SetRange(\"Day Filter\", 1) (FlowFilter, field 6), then SetRange(Total, 0) (FlowField, field 5), GetFilters gives `Day Filter: 1, Total: 0` (:582, :598, :615).",
    probe: "Not written: GetFilters order over field classes (Normal, FlowField, FlowFilter) crossed with the order the filters are set, a filter set twice, the field numbers against the declaration order, a field of a table extension, filters in other filter groups, a temporary record, Record and RecordRef, GetView, SetView and CopyFilters, and the filter list of a Find error (al-runner issue 8296).",
    awaiting: Awaiting {
        issues: &[8296],
        files: &["tests/al-language/record/filters/ByValueRecordView.al"],
        since: "2026-11-19",
    },
}
