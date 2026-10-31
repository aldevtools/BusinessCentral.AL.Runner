GroupEnabledDisablesField => {
    id: "group-enabled-disables-field",
    assumed: "A control inside a group whose Enabled reads No is disabled whatever its own Enabled says on a page type other than a card or a request page, and for a group Enabled bound to an expression. Pinned: a request page (misc/lt-object-properties-runtime-effect/ReportRequestPageGroupEditable.al:P194_ReportGroup_EditableEnabled) and a card's field with or without an Enabled of its own in a group with `Enabled = false` (pages/lt-ap-testpage-642/ApTestPageTests.al:895 Ap16_GroupEnabledFalse_Card, :1082 Ap16_GroupEnabledFalse_FieldEnabledTrue).",
    tests: "No Microsoft test: a partner app's card test (a field with Enabled = true inside a group with Enabled = false reads Enabled No on the TestPage).",
    probe: "A list and a document page with a field (Enabled = true, and without Enabled) in a group with Enabled = false, and with the group's Enabled bound to a variable that OnOpenPage sets: Enabled() of the field on the TestPage.",
}
