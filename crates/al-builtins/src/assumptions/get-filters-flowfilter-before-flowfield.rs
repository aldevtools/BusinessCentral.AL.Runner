GetFiltersFlowfilterBeforeFlowfield => {
    id: "get-filters-flowfilter-before-flowfield",
    assumed: "Record.GetFilters of a record in filter group 0, not on MarkedOnly, whose filter group holds exactly two filters, one on a FlowField and one on a FlowFilter field with a higher field number, the FlowFilter's set first, lists the FlowFilter first. Every other shape with a FlowField filter before a FlowFilter filter in field order refuses: the filters set in the other order, more filters, another filter group, MarkedOnly, RecordRef.GetFilters and the filter list of a Find error.",
    tests: "Corpus PR 1430 (merged 2026-11-18, merge commit 75d895e1, not in the pinned corpus yet): tests/al-language/record/filters/ByValueRecordView.al:569-616 (ByValue_FlowFieldFilter_Codeunit, ByValue_FlowFieldFilter_LocalProcedure, ByValue_TableProcedure_OwnRecByValue): SetRange(\"Day Filter\", 1) (FlowFilter, field 6), then SetRange(Total, 0) (FlowField, field 5), GetFilters gives `Day Filter: 1, Total: 0` (:582, :598, :615).",
    probe: "Not written: GetFilters order over field classes (Normal, FlowField, FlowFilter) crossed with the order the filters are set, the field numbers, Record and RecordRef, and the filter list of a Find error (al-runner issue 8296).",
    awaiting: Awaiting {
        issues: &[8296],
        files: &["tests/al-language/record/filters/ByValueRecordView.al"],
        since: "2026-11-19",
    },
}
