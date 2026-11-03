SetValueWholeNumberIntoTextControl => {
    id: "setvalue-whole-number-into-text-control",
    assumed: "TestPage SetValue of a BigInteger, or of a Decimal that is not a whole number below 1000, on a Text or Code control bound to a variable enters the number as Format writes it. (Pinned and not assumed: an Integer and the Decimal 14.0 enter their digits, pages/lt-ao-testpage/AoTests.al NumberIntoTextVariable and NumberIntoCodeVariable.)",
    tests: "SCM RTAM Item Tracking-II (RegisterWhseJournalLineWithSNWarehouseTracking): WhseItemTrackingPageHandler sets Serial No. to a Decimal.",
    probe: "SetValue(1234.0), SetValue(1.5) and SetValue of a BigInteger on a Text[30] and a Code[30] control bound to a variable.",
}
