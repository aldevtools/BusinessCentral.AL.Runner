GroupTwoEnumFlowFilterRelation => {
    id: "group-two-enum-flowfilter-relation",
    assumed: "With the active filter group reset to 0 and a default source buffer, one exact enum FlowFilter in group 2 constrains TableRelation validation.",
    tests: "AL Runner issue 7877, measured on all three supported BC majors. The issue records the exact builds.",
    probe: "record/filters/TestRecordEnumFlowFilterGroup.al, corpus pull request 1341. FieldRef.Validate, a temporary source and an existing TestPage row are unmeasured.",
    awaiting: Awaiting {
        issues: &[7877],
        files: &["tests/al-language/record/filters/TestRecordEnumFlowFilterGroup.al"],
        since: "2026-11-12",
    },
}
