LookupLinkDefersRefreshWithSecondPart => {
    id: "lookup-link-defers-refresh-with-second-part",
    assumed: "On a host with row triggers whose ListPart links three fields, a TestField.Lookup whose OnValidate calls CurrPage.Update(false) also leaves the refresh for later when the host has a second part: an unlinked ListPart declared before the linked one, or a CardPart in the FactBoxes area with a one-field link. The Lookup and the reads after it run no row trigger; every part keeps what it showed. Measured on SaaS (newest major) by corpus writer Y63 (observe form); not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6639, the reporter's SecondListPartShowsThePickedResource and CardFactBoxShowsThePickedResource.",
    probe: "Y15LookupThreeFieldLink.al Y63_* on corpus branch al-runtime-rs-staging-y63 (observe commit 3d50121d).",
    awaiting: Awaiting {
        issues: &[6639],
        files: &[
            "tests/al-language/pages/lt-y15/Y15LookupThreeFieldLink.al::Y63_TwoPart_LookupReadEachControl",
            "tests/al-language/pages/lt-y15/Y15LookupThreeFieldLink.al::Y63_CardPart_LookupReadEachControl",
        ],
        since: "2026-11-01",
    },
}
