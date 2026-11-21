FeatureKeyRowsContainerMeasured => {
    id: "feature-key-rows-container-measured",
    assumed: "On every Business Central 29.1 build and in every country, the Feature Key table has exactly 14 rows: the 13 rows of 29.0 unchanged (record/PFFeatureKeyColumns.al RowsE) and NewNotificationBar (Enabled All Users, Mandatory By Version 30.0, Can Try Yes, Is One Way No, Data Update Required No), and no other key; so every read of the table on 29.1 rests on it, also one that finds no row. Measured on one build in one country only (29.1.57326.58379, W1). The list is served for every 29.1 build, and the runtime does not see the country. The other builds and countries are assumed to match because on 28 neither changed the list: the same probe on a 28.4 DE container read the rows the corpus pins for 28.1 to 28.5 (RowsD) unchanged, except SalesPrices, which that tenant had switched on by hand.",
    tests: "No Microsoft test in the runner's suites. al-runner issue 8192: `Feature Management Impl.`.IsEnabled is refused on 29.1, in Company-Initialize too, so the install baseline stops. A contributor's measurement on one Business Central 29.1 W1 sandbox container (build 29.1.57326.58379, fresh tenant, 2026-11-18) with the RowText and the field list of record/PFFeatureKeyColumns.al: the same 10 fields, 14 rows. The same probe on a 28.4 DE container read RowsD unchanged but for SalesPrices (switched on by hand in that tenant).",
    probe: "record/PFFeatureKeyColumns.al on CI legs of Business Central 29.1, W1 and DE (a 29.1 branch in ExpectedRows; the maintainers write it). Not measured, so refused on 29.1: the Description, Learn More Link and Description In English columns. Not measured either: other 29.1 builds than 29.1.57326.58379, other countries than W1.",
    awaiting: Awaiting {
        issues: &[8192],
        files: &["tests/al-language/record/PFFeatureKeyColumns.al::FeatureKey_RowsPerId_29_1"],
        since: "2026-11-19",
    },
}
