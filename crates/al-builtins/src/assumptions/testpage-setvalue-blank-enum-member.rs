TestPageSetValueBlankEnumMember => {
    id: "testpage-setvalue-blank-enum-member",
    assumed: "TestPageField.SetValue of an Integer on a control bound to an Enum page variable selects the value of that ordinal when its caption is blank (all spaces), on a list, a page without a source table or a page without a row. A card with a source row is pinned (pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:2366).",
    tests: "Tests-ERM ERM Budget (134922, 36 tests) and ERM Budget UI (134927, 11 tests): Initialize calls Budget.GLAccCategory.SetValue(0) on page 113 Budget, a control over GLAccCategoryFilter: Enum \"G/L Account Category\" (value 0 \" \", Caption ' ').",
    probe: "The BeEnumSetValueBlank cases (SetValue(0) on a blank-captioned Enum variable control) on a page without a source table and on a list page.",
}
