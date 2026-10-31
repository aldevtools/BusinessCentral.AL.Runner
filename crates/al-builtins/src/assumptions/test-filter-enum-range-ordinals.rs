TestFilterEnumRangeOrdinals => {
    id: "test-filter-enum-range-ordinals",
    assumed: "TestFilter.GetFilter of an Enum field filtered to a range over more than two values (SetRange on the record handed to the page, or a SetFilter text range) reads as the two ordinals, `0..2`, as a range of two values does (pages/lt-cd-testpage-leads/CdTestPageLeads.al:923 SH_TestPageFilter_GetFilter_EnumRangeAndList `0..1`, `1..2`).",
    tests: "No Microsoft test: a partner app's test on its own list page.",
    probe: "TestFilter.GetFilter of an Enum field after SetRange(First, Third) on the record handed to Page.Run, and after a SourceTableView range over three values.",
}
