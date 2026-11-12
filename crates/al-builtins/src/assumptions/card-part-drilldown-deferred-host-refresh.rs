CardPartDrilldownDeferredHostRefresh => {
    id: "card-part-drilldown-deferred-host-refresh",
    assumed: "After a CardPart with UpdatePropagation Both changes the shared stored header in a drilldown and calls Update(false), the Document still shows the old value and rereads the header before its next SetValue.",
    tests: "Local neutral CardPartRefresh.al probe.",
    probe: "Add the same-table CardPart drilldown and next SetValue to the pinned corpus.",
}
