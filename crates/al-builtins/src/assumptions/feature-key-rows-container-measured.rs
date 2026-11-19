FeatureKeyRowsContainerMeasured => {
    id: "feature-key-rows-container-measured",
    assumed: "On Business Central 29.1 the Feature Key table has exactly 14 rows: the 13 rows of 29.0 unchanged (record/PFFeatureKeyColumns.al RowsE) and NewNotificationBar (Enabled All Users, Mandatory By Version 30.0, Can Try Yes, Is One Way No, Data Update Required No), and no other key; so every read of the table on 29.1 rests on it, also one that finds no row. The corpus measured other minors of the three majors (record/PFFeatureKeyColumns.al), not 29.1.",
    tests: "No Microsoft test in the runner's suites. al-runner issue 8192: `Feature Management Impl.`.IsEnabled is refused on 29.1, in Company-Initialize too, so the install baseline stops. A contributor's measurement on one Business Central 29.1 W1 sandbox container (build 29.1.57326.58379, fresh tenant, 2026-11-18) with the RowText and the field list of record/PFFeatureKeyColumns.al: the same 10 fields, 14 rows; the same probe read the 28.4 rows of RowsD unchanged.",
    probe: "record/PFFeatureKeyColumns.al on a CI leg of Business Central 29.1 (a 29.1 branch in ExpectedRows, and the test FeatureKey_RowsPerId_29_1). Not measured, so refused on 29.1: the Description, Learn More Link and Description In English columns. Not measured either: other countries than W1, other 29.1 builds.",
    awaiting: Awaiting {
        issues: &[8192],
        files: &["tests/al-language/record/PFFeatureKeyColumns.al::FeatureKey_RowsPerId_29_1"],
        since: "2026-11-19",
    },
}
