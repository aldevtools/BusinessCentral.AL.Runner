IntegerRecordFieldInAndBoolean => {
    id: "integer-record-field-in-and-boolean",
    assumed: "For a qualified Integer record field tested for membership in an Integer set followed by AND, membership binds first and the Boolean right operand is always evaluated.",
    tests: "Local neutral IntegerMembership.al probe.",
    probe: "Add the record field and eager evaluation cases to the pinned corpus.",
}
