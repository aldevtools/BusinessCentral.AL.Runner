TestPageSetValueBlankOptionMember => {
    id: "testpage-setvalue-blank-option-member",
    assumed: "TestPageField.SetValue of a blank text on an Option control whose first member has no OptionCaption (the member name is blank) selects that member. A control with an OptionCaption starting with ' ' is pinned.",
    tests: "Tests-Marketing Marketing Matrix Management.TaskWithStatus* (Tasks.StatusFilter.SetValue(StatusFilter::\" \") resets the filter); 7 tests.",
    probe: "A page with an Option variable control without OptionCaption whose first member is \" \": TestPage SetValue(' ') after SetValue(A), then Value.",
}
