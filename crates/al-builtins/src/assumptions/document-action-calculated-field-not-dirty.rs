DocumentActionCalculatedFieldNotDirty => {
    id: "document-action-calculated-field-not-dirty",
    assumed: "On a clean Document page, a change only to a calculated field in Rec during an action does not cause an implicit page save, including when the action committed a write through a separate record buffer.",
    tests: "AL Runner issue 7882, measured on all three supported BC majors. The issue records the exact builds.",
    probe: "Corpus pull request 1346.",
}
