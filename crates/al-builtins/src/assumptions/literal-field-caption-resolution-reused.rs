LiteralFieldCaptionResolutionReused => {
    id: "literal-field-caption-resolution-reused",
    assumed: "Repeated Caption reads of a control over the same literal field CaptionClass reuse its resolved caption, including after an unrelated control edit.",
    tests: "Local neutral probe LiteralCaption.al.",
    probe: "Add a literal field CaptionClass with repeated reads and a subscriber count to the pinned corpus.",
}
