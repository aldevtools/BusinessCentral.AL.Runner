FieldRowExtensionPackageIds => {
    id: "field-row-extension-package-ids",
    assumed: "The Field row of a field a table extension adds has the extension's App Package ID and App Runtime Package ID (its AllObj TableExtension row's), not the table's, after Get, after FindFirst on the key and in a scan, and a read filtered on App Package ID <> the table's keeps the extension fields and drops the table's own. Covered only for a table extension of an app under test (not a dependency app compiled from its package), on a table of an app under test or of Base Application. A table extension of a dependency app leaves the two columns unknown, so a read of them is refused. A Get of the table's own fields still gives null ids (metadata/platform-tables/PlatformMetadataColumnValues.al:287-311).",
    tests: "No Microsoft test. An outside report: al-runner issue 8051, the reporter's repro (codeunit 50120, 3/3 tests pass on BC 29.1.57326.57497) and a partner app that maps attributes to the fields other extensions add.",
    probe: "Open corpus PR 1402 (tests/al-language/metadata/platform-tables/PlatformMetadataColumnValues.al, two tests at the end of codeunit 71394): cross-app fixture B's field 61110 and this app's field 83950 on fixture A's table 61101, and this app's field 96160 on Item, after Get, FindFirst on the key and a scan; and the filter on App Package ID <> the table's. Measured with the same shapes on a local BC 29.1.57326.57497 (OnPrem w1). Not measured: a table extension of a Microsoft app.",
    awaiting: Awaiting {
        issues: &[8051],
        files: &[
            "tests/al-language/metadata/platform-tables/PlatformMetadataColumnValues.al::FieldTable_AppPackageIds_OfTableExtensionField_AreTheExtensionApps",
            "tests/al-language/metadata/platform-tables/PlatformMetadataColumnValues.al::FieldTable_FilterAppPackageIdNotTheTables_KeepsOnlyExtensionFields",
        ],
        since: "2026-11-14",
    },
}
