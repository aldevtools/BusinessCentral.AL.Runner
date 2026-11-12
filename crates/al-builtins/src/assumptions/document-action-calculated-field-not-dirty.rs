DocumentActionCalculatedFieldNotDirty => {
    id: "document-action-calculated-field-not-dirty",
    assumed: "On a clean Document page, a change only to a calculated field in Rec during an action does not cause an implicit page save, including when the action committed a write through a separate record buffer.",
    tests: "Local neutral ReworkSave.al probe.",
    probe: "Add the Document action and committed separate-buffer write to the pinned corpus.",
}
