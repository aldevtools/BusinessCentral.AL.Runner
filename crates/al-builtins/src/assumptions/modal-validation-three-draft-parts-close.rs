ModalValidationThreeDraftPartsClose => {
    id: "modal-validation-three-draft-parts-close",
    assumed: "After a modal field validation error on a clean non-lookup Document with three clean draft ListParts, an allowed close asks the ListParts with None and the Document with OK, then preserves the validation error.",
    tests: "Local neutral probe ModalThreeParts.al.",
    probe: "Add the draft ListParts and close-trigger trace to the pinned corpus.",
}
