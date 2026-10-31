TestPageAssertEqualsIntegerOnTextControl => {
    id: "testpage-assertequals-integer-on-text-control",
    assumed: "TestPageField.AssertEquals(<BigInteger>) on a control over a Text or Code value compares the number spelled in plain digits with the control's text, for a BigInteger outside 0 to 9 (an Integer and a Decimal are pinned). An Enum there stays refused.",
    tests: "Tests-Marketing Marketing Matrix Management.TasksMatrixHandlerWithValue (Field1.AssertEquals(1) on the Text[1024] cells of Tasks Matrix); the 10 TaskWith* tests.",
    probe: "A page with a field over a Text[30] variable holding '1000' and '-5': TestPage Field.AssertEquals(1000), AssertEquals(-5) with a BigInteger expected value.",
}
