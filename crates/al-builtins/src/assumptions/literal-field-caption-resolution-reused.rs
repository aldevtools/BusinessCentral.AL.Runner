LiteralFieldCaptionResolutionReused => {
    id: "literal-field-caption-resolution-reused",
    assumed: "Repeated Caption reads of a control over the same literal field CaptionClass reuse its resolved caption, including after an unrelated control edit.",
    tests: "AL Runner issue 7884, measured on all three supported BC majors. The issue records the exact builds.",
    probe: "Corpus pull request 1356.",
}
