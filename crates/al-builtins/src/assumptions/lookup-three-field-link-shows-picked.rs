LookupThreeFieldLinkShowsPicked => {
    id: "lookup-three-field-link-shows-picked",
    assumed: "A TestField.Lookup through OnLookup whose OnValidate calls CurrPage.Update(false), on a host without row triggers, shows the picked value in a control the page does not keep when the page's ListPart has a SubPageLink of three `field(..)` entries, as with one or two (pinned by pages/lt-sc-outside-pages/ScLookupGlobalRecordField.al and pages/lt-wb-6290/WbLookupListPartTests.al:21-31). With three entries BC runs the refresh after the Lookup later (pages/lt-y15/Y15LookupThreeFieldLink.al:194 `resource no` `R2`, :278), which a host without row triggers does not show.",
    tests: "No Microsoft test: an outside report (the three-field link page shows R2 after the lookup on real BC).",
    probe: "Pinned with a host that has OnAfterGetCurrRecord: pages/lt-y15/Y15LookupThreeFieldLink.al (its 4 tests are refused here, because this runtime refreshes within the Lookup). Wanted: the same page without OnAfterGetRecord and OnAfterGetCurrRecord on the host, `resource no` after the Lookup.",
}
