LookupThreeFieldLinkShowsPicked => {
    id: "lookup-three-field-link-shows-picked",
    assumed: "A TestField.Lookup through OnLookup whose OnValidate calls CurrPage.Update(false) shows the picked value in a control the page does not keep when the page's ListPart has a SubPageLink of three `field(..)` entries, as with one or two (pinned by pages/lt-sc-outside-pages/ScLookupGlobalRecordField.al and pages/lt-wb-6290/WbLookupListPartTests.al:21-31).",
    tests: "No Microsoft test: al-runner issue 6466, the reporter's run on real BC (the three-field link page shows R2 after the lookup). Measured by corpus writer Y15 (Y15LookupThreeFieldLink.al:194, on the SaaS sandbox, newest major: `resource no` `R2`); a host with row triggers is refused, because BC runs the refresh after the Lookup later (:278).",
    probe: "Y15LookupThreeFieldLink.al on branch al-runtime-rs-staging-y15.",
    awaiting: Awaiting {
        issues: &[6466],
        files: &["tests/al-language/pages/lt-y15/Y15LookupThreeFieldLink.al"],
        since: "2026-10-31",
    },
}
