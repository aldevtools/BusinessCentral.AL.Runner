GroupTwoEnumFlowFilterRelation => {
    id: "group-two-enum-flowfilter-relation",
    assumed: "With the active filter group reset to 0 and a default source buffer, one exact enum FlowFilter in group 2 constrains TableRelation validation.",
    tests: "AL Runner issue 7877, measured on BC 27.5.46862.56996, 28.5.54151.57550 and 29.0.54011.57566.",
    probe: "record/filters/TestRecordEnumFlowFilterGroup.al, corpus pull request 1341. FieldRef.Validate, a temporary source and an existing TestPage row are unmeasured.",
}
