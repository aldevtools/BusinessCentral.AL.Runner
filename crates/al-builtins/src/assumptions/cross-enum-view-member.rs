CrossEnumViewMember => {
    id: "cross-enum-view-member",
    assumed: "A member of another enum in a page's view property, `const(\"Purchase Document Type\"::Order)` on a field of enum \"Purchase Comment Document Type\", reads as the field's member of the same name and ordinal. Pinned: the part shows the Order comment and not the Invoice one (record/qualified-enum-filters/TestQualifiedEnumFilters.al:162, corpus pull request 1199), where name and ordinal agree. Whether BC matches by name or by ordinal is not pinned: a member whose name or ordinal differs stays refused.",
    tests: "No Microsoft test: al-runner issue 7265 (the reporter's real-BC run of SubPageView_CrossEnumQualifier_SelectsPurchaseOrderComment on the three supported majors, corpus pull request 1199).",
    probe: "record/qualified-enum-filters/TestQualifiedEnumFilters.al once merged; also wanted: a member of another enum whose name matches but ordinal differs, and one whose ordinal matches but name differs.",
    awaiting: Awaiting {
        issues: &[7265],
        files: &["tests/al-language/record/qualified-enum-filters/TestQualifiedEnumFilters.al::SubPageView_CrossEnumQualifier_SelectsPurchaseOrderComment"],
        since: "2026-11-08",
    },
}
