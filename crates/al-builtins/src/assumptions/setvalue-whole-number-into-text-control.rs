SetValueWholeNumberIntoTextControl => {
    id: "setvalue-whole-number-into-text-control",
    assumed: "TestPage SetValue of an Integer, BigInteger or whole Decimal (below 1000 in size) on a Text or Code control enters the number's digits.",
    tests: "SCM RTAM Item Tracking-II (RegisterWhseJournalLineWithSNWarehouseTracking): WhseItemTrackingPageHandler sets Serial No. to a Decimal.",
    probe: "SetValue(14), SetValue(14.0), SetValue(1234) and SetValue(-3) on a Text[30] and a Code[30] control bound to a field and to a variable. Asked in corpus #641.",
}
