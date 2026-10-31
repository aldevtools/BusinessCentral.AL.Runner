LookupTwoFieldLinkDefersRefresh => {
    id: "lookup-two-field-link-defers-refresh",
    assumed: "A TestField.Lookup through OnLookup whose OnValidate calls CurrPage.Update(false), on a page with OnAfterGetRecord or OnAfterGetCurrRecord whose ListPart has a SubPageLink of two `field(..)` entries, runs no row trigger of the host or the part; the reads after it run none either: the field shows the picked value, the globals the row triggers set and the part keep their values. Every later call but a read and Close is refused.",
    tests: "No Microsoft test: al-runner issue 6519. Measured by corpus writer Y26 on the SaaS sandbox, newest major (Y15LookupThreeFieldLink.al:379 Y26_TwoLinkCurr_LookupRefresh, :509 Y26_TwoLinkPlain_LookupRefresh).",
    probe: "Y15LookupThreeFieldLink.al on branch al-runtime-rs-staging-y26.",
    awaiting: Awaiting {
        issues: &[6519],
        files: &["tests/al-language/pages/lt-y15/Y15LookupThreeFieldLink.al"],
        since: "2026-10-31",
    },
}
