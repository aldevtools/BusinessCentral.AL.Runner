LookupTwoFieldLinkDefersRefresh => {
    id: "lookup-two-field-link-defers-refresh",
    assumed: "A TestField.Lookup through OnLookup whose OnValidate calls CurrPage.Update(false), on a page with OnAfterGetRecord or OnAfterGetCurrRecord whose ListPart has a SubPageLink of two `field(..)` entries, runs no row trigger of the host or the part; the reads after it run none either: the field shows the picked value, the globals the row triggers set and the part keep their values. Every later call but a read and Close is refused.",
    tests: "No Microsoft test: an outside report. Measured by corpus writer Y26 on the SaaS sandbox, newest major (Y15LookupThreeFieldLink.al:379 Y26_TwoLinkCurr_LookupRefresh, :509 Y26_TwoLinkPlain_LookupRefresh).",
    probe: "Pinned by pages/lt-y15/Y15LookupThreeFieldLink.al (corpus pin 92031e20), but this runtime refuses its tests (a control computed when it is read after the Lookup). Retire once they run.",
}
