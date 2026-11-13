ModalValidationThreeDraftPartsClose => {
    id: "modal-validation-three-draft-parts-close",
    assumed: "The measured modal Document closes its three clean linked ListPart drafts after a page field validation error.",
    tests: "al-runner issue 7891 and corpus PR #1353, pages/close-errors/TestModalThreeDraftParts.al (BC 27.5, 28.5 and 29.0).",
    probe: "Corpus PR #1353, pages/close-errors/TestModalThreeDraftParts.al.",
    awaiting: Awaiting {
        issues: &[7891],
        files: &["tests/al-language/pages/close-errors/TestModalThreeDraftParts.al"],
        since: "2026-11-13",
    },
}
