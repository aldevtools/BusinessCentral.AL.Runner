LookupLinkDefersRefreshWithManyParts => {
    id: "lookup-link-defers-refresh-with-many-parts",
    assumed: "On a host with row triggers whose ListPart links three fields, a TestField.Lookup whose OnValidate calls CurrPage.Update(false) leaves the refresh for later also with three or four parts: two or three unlinked ListParts before, between or after the linked one (four: linked last), with or without a FactBox CardPart linked by one field (three ListParts). The Lookup and the reads after it run no row trigger and every part keeps what it showed; the first access to a part no call reached runs the open's reads again. Measured on a Windows BC container by corpus writer R6806; not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6806, the reporter's ThreePartsShowThePickedResource and FourPartsShowThePickedResource.",
    probe: "R6806LookupParts.al R6806_* on corpus branch al-runtime-rs-staging-r6806 (the reporter's shape: R6806_ReporterABL_LookupShowsPickedResource, R6806_ReporterABLK_LookupShowsPickedResource).",
    awaiting: Awaiting {
        issues: &[6806],
        files: &[
            "tests/al-language/pages/lt-r6806-lookup-parts/R6806LookupParts.al::R6806_ABL_LookupReadEachControl",
            "tests/al-language/pages/lt-r6806-lookup-parts/R6806LookupParts.al::R6806_ReporterABL_LookupShowsPickedResource",
        ],
        since: "2026-11-02",
    },
}
