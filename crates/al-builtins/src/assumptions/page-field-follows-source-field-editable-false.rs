PageFieldFollowsSourceFieldEditableFalse => {
    id: "page-field-follows-source-field-editable-false",
    assumed: "A page field over a source table field that is a FlowFilter and declares Editable = false, with no Editable of its own on the control, reads Editable() No on a TestPage. A stored field (pages/lt-page-field-editable-source/PageFieldEditableSource.al) and a FlowField (handlers/N582TestPageEdges.al:324) are pinned.",
    tests: "Payment Export Mgt Unit Test.FilterByHasPaymentExportError (Gen. Journal Line.\"Has Payment Export Error\", a FlowField with Editable = false, on the Payment Journal).",
    probe: "A FlowFilter field with Editable = false and a page with a plain field control over it, and one that sets Editable = true itself: Editable() of each control on an open TestPage.",
}
