CardPartDrilldownDeferredHostRefresh => {
    id: "card-part-drilldown-deferred-host-refresh",
    assumed: "After a CardPart with UpdatePropagation Both changes the shared stored header in a drilldown and calls Update(false), the Document still shows the old value and rereads the header before its next SetValue.",
    tests: "AL Runner issue 7886; corpus pull request 1348.",
    probe: "Corpus pull request 1348.",
}
