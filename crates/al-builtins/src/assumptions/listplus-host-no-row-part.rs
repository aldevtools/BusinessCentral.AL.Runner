ListplusHostNoRowPart => {
    id: "listplus-host-no-row-part",
    assumed: "A ListPlus host opened for view on no row runs its list part as a Document or Card host does: the part runs OnNewRecord once (blank link value, BelowxRec Yes) and OnAfterGetCurrRecord once on the blank row, and the host runs neither.",
    tests: "No Microsoft test; al-runner issue 6953 (TestPage OpenView of Allocation Account and Bank Acc. Reconciliation, both ListPlus). Measured on BC 29 (Linux CI) by the probe below.",
    probe: "pages/parts/PartListPlusHostNoRow.al on corpus branch al-runtime-rs-staging-r6951 (ListPlusHost_OpenView_NoRow_PartTriggers, _PartInGroup, _HiddenPart, _PartFirst).",
    awaiting: Awaiting {
        issues: &[6953],
        files: &["tests/al-language/pages/parts/PartListPlusHostNoRow.al"],
        since: "2026-11-04",
    },
}
