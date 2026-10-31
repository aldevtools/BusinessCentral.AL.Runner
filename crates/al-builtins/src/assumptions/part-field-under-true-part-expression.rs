PartFieldUnderTruePartExpression => {
    id: "part-field-under-true-part-expression",
    assumed: "TestField.Editable()/Enabled() of a field of a part whose part control has an Editable or Enabled expression that comes out true answer as without the property. An expression that comes out false is refused; a literal Editable = false is pinned (PnGPartRows.al:117).",
    tests: "Tests-ERM sales and purchase document tests that read fields of a lines part behind `Editable = IsSalesLinesEditable; Enabled = IsSalesLinesEditable;` (and the purchase twins), about 170 passes in Tests-ERM.",
    probe: "A host card with parts `Editable = Flag` and `Enabled = Flag`: with Flag false and true (set in OnOpenPage, then changed by an action), a part field's Editable() and Enabled(), and SetValue through the part.",
}
