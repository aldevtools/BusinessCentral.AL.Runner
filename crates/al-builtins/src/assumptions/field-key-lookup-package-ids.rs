FieldKeyLookupPackageIds => {
    id: "field-key-lookup-package-ids",
    assumed: "A key lookup of a Field row (a Get, or a FindFirst or FindSet whose filters set TableNo and No. to one value each) of a field the table declares gives null App Package ID and App Runtime Package ID unless a table extension of the table's own app extends the table; then, and for a system field (2000000000 to 2000000004) and a field another app's table extension adds, it gives the row's ids as a scan does. Marked where the answer differs from the earlier rule (null exactly for a table of the app under test, on a Get only): a dependency table without a same-app extension, a table of the app under test with one, a system field of the app under test, and a key FindFirst or FindSet. A key lookup that also filters on a package column, of a row the lookup gives null ids, is refused (Business Central answers its FindFirst with no row while its Count is 1).",
    tests: "No Microsoft test. Outside: the follow-up probes of al-runner issue 8051 (PR 8054), 18 tables of six apps on Business Central 29.1.57326.57497.",
    probe: "Corpus PRs 1404 and 1407, merged on corpus master (276cf87e), not pinned yet: tests/al-language/metadata/platform-tables/PlatformMetadataColumnValues.al:516-532 (FieldTable_AppPackageIds_GetOfOwnField_NullForSystemApplicationAndFixtureTables), :569-605 (FieldTable_AppPackageIds_KeyLookupOfOwnField_NullUnlessSameAppTableExtension) and :618-646 (FieldTable_AppPackageIds_GetOfSystemField_IsTheTablesWithoutSameAppTableExtension), green on every BC major of their CI. Not measured: a key lookup with a package filter, and a same-app extension that declares no field.",
    awaiting: Awaiting {
        issues: &[8051],
        files: &[
            "tests/al-language/metadata/platform-tables/PlatformMetadataColumnValues.al::FieldTable_AppPackageIds_GetOfOwnField_NullForSystemApplicationAndFixtureTables",
            "tests/al-language/metadata/platform-tables/PlatformMetadataColumnValues.al::FieldTable_AppPackageIds_KeyLookupOfOwnField_NullUnlessSameAppTableExtension",
            "tests/al-language/metadata/platform-tables/PlatformMetadataColumnValues.al::FieldTable_AppPackageIds_GetOfSystemField_IsTheTablesWithoutSameAppTableExtension",
        ],
        since: "2026-11-14",
    },
}