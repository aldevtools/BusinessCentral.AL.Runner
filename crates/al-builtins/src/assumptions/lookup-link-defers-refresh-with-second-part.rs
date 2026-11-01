LookupLinkDefersRefreshWithSecondPart => {
    id: "lookup-link-defers-refresh-with-second-part",
    assumed: "On a host with row triggers whose ListPart links three fields, a TestField.Lookup whose OnValidate calls CurrPage.Update(false) also leaves the refresh for later when the host has a second part: an unlinked ListPart declared before the linked one, or a CardPart in the FactBoxes area with a one-field link. The Lookup and the reads after it run no row trigger; every part keeps what it showed. Measured on SaaS (newest major) by corpus writer Y63.",
    tests: "No Microsoft test: an outside report, the reporter's SecondListPartShowsThePickedResource and CardFactBoxShowsThePickedResource.",
    probe: "Pinned by pages/lt-y15/Y15LookupThreeFieldLink.al Y63_* (corpus pin 9cf26907), but this runtime refuses all 14 Y63 tests: their pages show controls computed when they are read (HostFn, PartFn), and the refresh after TestField.Lookup of such a control is not served. Retire once they run.",
}
