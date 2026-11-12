DocumentActionCalculatedFieldNotDirty => {
    id: "document-action-calculated-field-not-dirty",
    assumed: "On a clean Document page, a change only to a calculated field in Rec during an action does not cause an implicit page save, including when the action committed a write through a separate record buffer.",
    tests: "AL Runner issue 7882, measured on BC 27.5.46862.56996, 28.5.54151.57550 and 29.0.54011.57566.",
    probe: "Corpus pull request 1346.",
}
