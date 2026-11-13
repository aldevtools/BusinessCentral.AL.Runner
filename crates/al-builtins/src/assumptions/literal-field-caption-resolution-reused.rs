LiteralFieldCaptionResolutionReused => {
    id: "literal-field-caption-resolution-reused",
    assumed: "A second Caption() of a control over the same literal field CaptionClass that raises OnResolveCaptionClass gives the caption of the first read without raising the event again, when the only test calls between the reads are Caption() reads and values typed into other controls of the page, and neither the language nor the bound subscriptions changed. Any other call between the reads keeps the refusal.",
    tests: "AL Runner issue 7884 (reporter's repro LiteralFieldCaptionReadsUseOneResolution, measured on all three supported BC majors; the issue records the exact builds).",
    probe: "Corpus pull request 1356.",
    awaiting: Awaiting {
        issues: &[7884],
        files: &["tests/al-language/record/captions/TestLiteralFieldCaptionResolution.al"],
        since: "2026-11-12",
    },
}
