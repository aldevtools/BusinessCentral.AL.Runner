SystemFieldRowDataClassification => {
    id: "system-field-row-data-classification",
    assumed: "The Field rows of SystemId (2000000000) and the audit fields (2000000001-2000000004) carry the platform's own DataClassification on every major, whatever the table declares: EndUserPseudonymousIdentifiers for SystemId, SystemCreatedBy and SystemModifiedBy, SystemMetadata for SystemCreatedAt and SystemModifiedAt; by Get and in a SetRange(TableNo) walk.",
    tests: "No Microsoft test: al-runner issue 6890 (a user report quoting another runner's reading of the platform's system-field helper).",
    probe: "R6890SystemFieldClassification.al on corpus branch al-runtime-rs-staging-r6890 (R6890_SystemFieldRows_*): measured on the two older majors on Windows and on the newest on the Linux CI.",
    awaiting: Awaiting {
        issues: &[6890],
        files: &["tests/al-language/metadata/platform-tables/R6890SystemFieldClassification.al"],
        since: "2026-11-03",
    },
}
