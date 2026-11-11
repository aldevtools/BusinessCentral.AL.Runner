ExactSourceFlowFilterRelation => {
    id: "exact-source-flowfilter-relation",
    assumed: "TableRelation validation treats an unset source FlowFilter as unrestricted and uses a single exact source FlowFilter value when it is set.",
    tests: "External repro in al-runner issue 7769, measured on every supported BC major.",
    probe: "The same tests in the pinned corpus.",
    awaiting: Awaiting {
        issues: &[7769],
        files: &["tests/al-language/record/filters/RelationWhereFlowFilter.al::Record_Validate_SourceFlowFilterUnset_AcceptsExistingNonDefaultKind", "tests/al-language/record/filters/RelationWhereFlowFilter.al::Record_Validate_SourceFlowFilterExactMatch_AcceptsTarget", "tests/al-language/record/filters/RelationWhereFlowFilter.al::Record_Validate_SourceFlowFilterMismatch_RejectsTarget"],
        since: "2026-11-11",
    },
}
