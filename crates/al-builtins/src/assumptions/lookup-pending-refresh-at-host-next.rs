LookupPendingRefreshAtHostNext => {
    id: "lookup-pending-refresh-at-host-next",
    assumed: "On a host with OnAfterGetCurrRecord only and a ListPart linked on three fields, the host's own Next as the first call after a TestField.Lookup runs the refresh that the OnValidate's CurrPage.Update(false) left (host and part read the row they stand on), then moves. The part then follows the new row. Pinned for the returned values only (pages/lt-y15/Y15LookupThreeFieldLink.al:259 Y15_ThreeLink_RegionsSetPartFollowsRow `next` `true/R2/R2`); the log of that Next is not observed.",
    tests: "No Microsoft test: the corpus case above.",
    probe: "Y15_ThreeLink_RegionsSetPartFollowsRow with `Log.Take()` observed after the host's Next (expected `agcr R1;agcr R2` if the refresh runs first).",
}
