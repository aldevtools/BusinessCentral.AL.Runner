CardPartDrilldownDeferredHostRefresh => {
    id: "card-part-drilldown-deferred-host-refresh",
    assumed: "After a CardPart with UpdatePropagation Both changes the shared stored header in a drilldown and calls Update(false), the Document still shows the old value and rereads the header before its next SetValue. The host has no page extension or subscriber, and only reads of host fields come between the drilldown and the SetValue; any other call on the host or the part ends the reread, and a second drilldown sets none.",
    tests: "AL Runner issue 7886 (reporter's repro DrilldownRefreshesTheHeaderBeforeTheNextScan, measured on all three supported BC majors; the issue records the exact builds).",
    probe: "Corpus pull request 1348.",
    awaiting: Awaiting {
        issues: &[7886],
        files: &["tests/al-language/pages/parts/TestCardPartDrilldownRefresh.al"],
        since: "2026-11-12",
    },
}
