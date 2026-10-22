//! The registry of platform tables that have a native row provider (ADR-013 decision 7, ADR-019).
//!
//! A platform table is a table of the platform app `System` (the system tables, ids from
//! 2000000000). Its schema is the real declaration, taken as AL. Its rows are not AL: the platform
//! computes them, or stores them under native triggers. A platform table with no entry here is
//! refused wherever code reaches it, because a declared table with no behaviour would answer
//! empty, and silently wrong.
//!
//! This is the second id namespace of the builtin registry (ADR-008). The providers themselves
//! live in `al-vm` (`platform/`); this table is data so that `al-sema` can refuse at compile time.

/// How the rows of a platform table are provided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// Rows computed on read; the table cannot be written.
    Virtual(Virtual),
    /// An ordinary stored table, with its real declaration (ADR-013 decision 7).
    Stored,
}

/// The computed tables. One variant per table the runtime can answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Virtual {
    Integer,
    Date,
    Session,
    ActiveSession,
    AllObj,
    AllObjWithCaption,
    CodeunitMetadata,
    TableMetadata,
    Field,
    Key,
    FeatureKey,
    PageMetadata,
    PageControlField,
    TimeZone,
    WindowsLanguage,
    /// Every row of a fresh tenant is pinned (platform/lt-platform-rows-fresh/PlatformRowsLanguageSelection.al).
    LanguageSelection,
    /// Two rows are pinned (platform/lt-platform-tables/PtCloudTables.al:60-76); any other read is refused.
    LicensePermission,
    NavAppInstalledApp,
    NavAppExtra,
    InstalledApplication,
    PublishedApplication,
    /// Known to exist, not pinned: lowers, and every read is refused.
    ApiWebhookEntity,
    /// Media live in the session's media store (ADR-017); the table is not a view of it yet.
    TenantMedia,
    MetadataPermissionSet,
    PermissionSet,
    /// The assignable rows of Permission Set (the table is walked through this one); not a table.
    PermissionSetAssignable,
    AggregatePermissionSet,
    MetadataPermission,
    ExpandedPermission,
    Permission,
    ReportLayoutList,
    ReportMetadata,
    ReportDataItems,
    XmlPortMetadata,
    QueryMetadata,
    EventSubscription,
    PageAction,
    TableRelations,
    /// A table of the platform app that is not in [`REGISTRY`]: it lowers, a temporary record of it is
    /// an ordinary temporary table, and every read or write of its persistent rows is refused.
    Unprovided,
}

/// (table id, table name, provider). Every entry is a table of the platform `System` app.
pub const REGISTRY: &[(u32, &str, Provider)] = &[
    (2_000_000_026, "Integer", Provider::Virtual(Virtual::Integer)),
    (2_000_000_007, "Date", Provider::Virtual(Virtual::Date)),
    (2_000_000_009, "Session", Provider::Virtual(Virtual::Session)),
    (
        2_000_000_110,
        "Active Session",
        Provider::Virtual(Virtual::ActiveSession),
    ),
    (2_000_000_038, "AllObj", Provider::Virtual(Virtual::AllObj)),
    (
        2_000_000_058,
        "AllObjWithCaption",
        Provider::Virtual(Virtual::AllObjWithCaption),
    ),
    (
        2_000_000_137,
        "CodeUnit Metadata",
        Provider::Virtual(Virtual::CodeunitMetadata),
    ),
    (
        2_000_000_136,
        "Table Metadata",
        Provider::Virtual(Virtual::TableMetadata),
    ),
    (2_000_000_041, "Field", Provider::Virtual(Virtual::Field)),
    // Pinned for BC 27 only (record/TestFieldLenAndFeatureKeyContracts.al, `#if not BC28PLUS`).
    (2_000_000_211, "Feature Key", Provider::Virtual(Virtual::FeatureKey)),
    // Ordinary stored tables that take Insert and Delete (record/TestPlatformTablesWritableContracts.al:
    // Tenant License State refuses Delete, in al-vm records.rs).
    (2_000_000_196, "Object Options", Provider::Stored),
    (2_000_000_237, "Privacy Notice", Provider::Stored),
    (2_000_000_078, "Chart", Provider::Stored),
    (2_000_000_189, "Tenant License State", Provider::Stored),
    (2_000_000_063, "Key", Provider::Virtual(Virtual::Key)),
    (2_000_000_138, "Page Metadata", Provider::Virtual(Virtual::PageMetadata)),
    (
        2_000_000_192,
        "Page Control Field",
        Provider::Virtual(Virtual::PageControlField),
    ),
    (2_000_000_164, "Time Zone", Provider::Virtual(Virtual::TimeZone)),
    (
        2_000_000_045,
        "Windows Language",
        Provider::Virtual(Virtual::WindowsLanguage),
    ),
    (
        2_000_000_050,
        "Language Selection",
        Provider::Virtual(Virtual::LanguageSelection),
    ),
    (
        2_000_000_043,
        "License Permission",
        Provider::Virtual(Virtual::LicensePermission),
    ),
    (
        2_000_000_153,
        "NAV App Installed App",
        Provider::Virtual(Virtual::NavAppInstalledApp),
    ),
    (2_000_000_157, "NAV App Extra", Provider::Virtual(Virtual::NavAppExtra)),
    (
        2_000_000_212,
        "Installed Application",
        Provider::Virtual(Virtual::InstalledApplication),
    ),
    (
        2_000_000_206,
        "Published Application",
        Provider::Virtual(Virtual::PublishedApplication),
    ),
    (
        2_000_000_097,
        "API Webhook Entity",
        Provider::Virtual(Virtual::ApiWebhookEntity),
    ),
    // Stored tables whose rows AL reads as facts about the session; the platform seeds them
    // (al-vm `platform/seed.rs`).
    (2_000_000_006, "Company", Provider::Stored),
    (2_000_000_120, "User", Provider::Stored),
    (2_000_000_053, "Access Control", Provider::Stored),
    // Written by Record.AddLink and friends (al-vm `platform/links.rs`).
    (2_000_000_068, "Record Link", Provider::Stored),
    // Read by Base Application's API Webhook Notification Mgt. when a database write reaches the Global
    // Triggers codeunit; a plain tenant table with no native behaviour.
    (2_000_000_095, "API Webhook Subscription", Provider::Stored),
    (2000000159, "Data Sensitivity", Provider::Stored),
    (2000000073, "User Personalization", Provider::Stored),
    (
        2_000_000_250,
        "Metadata Permission Set",
        Provider::Virtual(Virtual::MetadataPermissionSet),
    ),
    (
        2_000_000_004,
        "Permission Set",
        Provider::Virtual(Virtual::PermissionSet),
    ),
    (
        2_000_000_167,
        "Aggregate Permission Set",
        Provider::Virtual(Virtual::AggregatePermissionSet),
    ),
    (2_000_000_165, "Tenant Permission Set", Provider::Stored),
    (2_000_000_166, "Tenant Permission", Provider::Stored),
    (2_000_000_253, "Tenant Permission Set Rel.", Provider::Stored),
    (
        2_000_000_251,
        "Metadata Permission",
        Provider::Virtual(Virtual::MetadataPermission),
    ),
    (
        2_000_000_254,
        "Expanded Permission",
        Provider::Virtual(Virtual::ExpandedPermission),
    ),
    (2_000_000_005, "Permission", Provider::Virtual(Virtual::Permission)),
    (2_000_000_178, "All Profile", Provider::Stored),
    (
        2_000_000_234,
        "Report Layout List",
        Provider::Virtual(Virtual::ReportLayoutList),
    ),
    (
        2_000_000_139,
        "Report Metadata",
        Provider::Virtual(Virtual::ReportMetadata),
    ),
    (
        2_000_000_203,
        "Report Data Items",
        Provider::Virtual(Virtual::ReportDataItems),
    ),
    (
        2_000_000_280,
        "XmlPort Metadata",
        Provider::Virtual(Virtual::XmlPortMetadata),
    ),
    (
        2_000_000_142,
        "Query Metadata",
        Provider::Virtual(Virtual::QueryMetadata),
    ),
    (
        2_000_000_140,
        "Event Subscription",
        Provider::Virtual(Virtual::EventSubscription),
    ),
    (2_000_000_143, "Page Action", Provider::Virtual(Virtual::PageAction)),
    (
        2_000_000_141,
        "Table Relations Metadata",
        Provider::Virtual(Virtual::TableRelations),
    ),
    (2_000_000_049, "Code Coverage", Provider::Stored),
    (2_000_000_288, "Code Coverage Test Lookup", Provider::Stored),
    (2_000_000_289, "Code Coverage Tests Run", Provider::Stored),
    (2_000_000_184, "Tenant Media", Provider::Virtual(Virtual::TenantMedia)),
    // An ordinary table: rows inserted, read by code (also lower case), duplicate Insert false, Count and DeleteAll
    // (platform/lt-platform-tables/PtCloudTables.al:303-360 QX_MediaResources_ImportStreamAndBlob). The rows a tenant
    // holds before any AL runs (:269) and the Tenant Media row of its media (:352) are refused elsewhere.
    (2_000_000_182, "Media Resources", Provider::Stored),
    (2_000_000_121, "User Property", Provider::Stored),
    (2_000_000_175, "Scheduled Task", Provider::Stored),
    (2_000_000_096, "API Webhook Notification", Provider::Stored),
    (2_000_000_098, "API Webhook Notification Aggr", Provider::Stored),
    // Take Insert, Modify and Delete (tests/al-language-onprem/session/PZPlatformSurvey.al:178-187). A fresh tenant
    // holds rows in them that nothing pins: see `UNSEEDED_ROWS`.
    (2_000_000_076, "Web Service", Provider::Stored),
    (2_000_000_168, "Tenant Web Service", Provider::Stored),
    // Empty on a fresh tenant and ordinary to Insert, Modify and Delete (platform/lt-platform-tables/PtCloudTables.al:
    // EntityText_InsertModifyDelete, ObjectAccessIntentOverride_InsertModifyDelete: `has_rows` No, `first_row` none).
    (2_000_000_132, "Entity Text", Provider::Stored),
    (2_000_000_205, "Object Access Intent Override", Provider::Stored),
    // Empty on a fresh tenant and ordinary to Insert, Modify and Delete, with several rows and a blank App ID (platform/lt-platform-tables/
    // PtServicesAppSetting.al: NavAppSetting_Content `count` 0, NavAppSetting_InsertModifyDelete, NavAppSetting_SeveralRowsAndBlankGuid).
    (2_000_000_201, "NAV App Setting", Provider::Stored),
];

/// Stored platform tables that a fresh tenant fills with rows no corpus test pins (Web Service and
/// Tenant Web Service: not empty, PZPlatformSurvey.al:178-187; 64 rows in W4PlatformAndMeta.al:93,
/// depending on the apps). The runtime starts them empty, so it treats them as unseeded
/// (`Engine::unseeded`): writes work, and a read of rows that were not written, or a `Get` that
/// finds nothing, is refused.
pub const UNSEEDED_ROWS: &[u32] = &[2_000_000_076, 2_000_000_168];

/// Platform tables that only the System app of a later major declares (the registry above is checked against the System app
/// of the first major the corpus pins, `tests/platform_tables.rs`). Ordinary stored tables here, written by install code:
/// "Agent Install" (System Application "Agent Upgrade") and "MCP Install" insert into them, and without a provider the install
/// baseline of those majors stopped at them and left tables unseeded. A Cloud-target `RecordRef.Open` of Agent Creation Control
/// is refused (platform/lt-aw-fresh-tenant/AwTests.al:590-616, `al-vm` refs.rs).
pub const LATER_MAJORS: &[(u32, &str, Provider)] = &[
    (2_000_000_297, "Agent Creation Control", Provider::Stored),
    (2_000_000_292, "MCP Configuration", Provider::Stored),
];

/// The provider of platform table `id`, if it has one.
pub fn provider(id: u32) -> Option<Provider> {
    REGISTRY
        .iter()
        .chain(LATER_MAJORS)
        .find(|(i, _, _)| *i == id)
        .map(|(_, _, p)| *p)
}

/// The computed-table provider of platform table `id`, if it is one.
pub fn virtual_table(id: u32) -> Option<Virtual> {
    match provider(id)? {
        Provider::Virtual(v) => Some(v),
        Provider::Stored => None,
    }
}

/// The columns of a registered table that a provider fills with a value something pins, by table
/// id. A table that is not listed has every column filled; for a table that is, a column outside
/// the list (and below the system fields) is not known: code that uses it is refused where it is
/// lowered (`al-sema`), never given a default (ADR-013 decision 7; hard rule 3), and the provider
/// leaves it `Value::Unknown` (`al-vm/src/platform/mod.rs`). Generated from the corpus
/// (`al_test_cli::platform_columns`): the columns a corpus file names, and `PINNED_ELSEWHERE`.
pub const PINNED_COLUMNS: &[(u32, &[u32])] = crate::pinned_columns::GENERATED;

/// The platform tables whose columns `PINNED_COLUMNS` lists.
pub const PINNED_TABLES: &[u32] = &[
    2_000_000_038,
    2_000_000_058,
    2_000_000_137,
    2_000_000_136,
    2_000_000_041,
    2_000_000_211,
    2_000_000_063,
    2_000_000_138,
    2_000_000_192,
    2_000_000_153,
    2_000_000_206,
    2_000_000_157,
    2_000_000_212,
    2_000_000_250,
    2_000_000_004,
    2_000_000_167,
    2_000_000_251,
    2_000_000_254,
    2_000_000_005,
    2_000_000_178,
    2_000_000_234,
    2_000_000_139,
    2_000_000_203,
    2_000_000_280,
    2_000_000_142,
    2_000_000_140,
    2_000_000_143,
    2_000_000_141,
    2_000_000_049,
    2_000_000_288,
    2_000_000_289,
    2_000_000_045,
    2_000_000_009,
    2_000_000_110,
];

/// Columns the corpus pins without naming them in a corpus file: read by number or by field name through a
/// `RecordRef` (often a loop over `FieldIndex`), through Microsoft's code, by `Get` on the primary key, or as the
/// target of a FlowField. Each as (table id, field id or name, the corpus test that pins the value the runtime
/// gives, as file:line under `corpus/tests/`). Where the provider answers only some rows (a declared property
/// that no test pins), the other rows hold `Value::Unknown`, and the comment says so.
pub const PINNED_ELSEWHERE: &[(u32, &str, &str)] = &[
    // Field. IsPartOfPrimaryKey of own and Base Application fields.
    (
        2_000_000_041,
        "28",
        "al-language/metadata/lt-platform-metadata-columns/PlatformMetadataColumnValues.al:237-268",
    ),
    // ExternalName blank and SQLDataType Varchar for Text fields; unknown for a declared ExternalName and for
    // SQLDataType of the other types (al-vm fields.rs).
    (2_000_000_041, "10", "al-language/record/PZFieldObsoleteRows.al:48"),
    (2_000_000_041, "23", "al-language/record/PZFieldObsoleteRows.al:52"),
    // Feature Key: Enabled and Mandatory By Version per feature and BC version, read by field name.
    (
        2_000_000_211,
        "2",
        "al-language/record/PFFeatureKeyColumns.al:33-35, :95-110",
    ),
    (
        2_000_000_211,
        "9",
        "al-language/record/PFFeatureKeyColumns.al:33-35, :95-110",
    ),
    // Published Application "Tenant ID": blank. Only read through the filter `'' | TenantId` of System
    // Application code, which blank and the tenant id both pass (al-vm apps.rs; docs/probes-needed.md row).
    (
        2_000_000_206,
        "12",
        "al-language/install/TestRetenPolAllowedTables_Tests.al:51-61",
    ),
    // The targets of the Lookup and Exist FlowFields of Published Application.
    (
        2_000_000_157,
        "1",
        "al-language-onprem/record/TestPublishedApplicationSysTable.al:294-365",
    ),
    (
        2_000_000_157,
        "2",
        "al-language-onprem/record/TestPublishedApplicationSysTable.al:294-365",
    ),
    (
        2_000_000_157,
        "3",
        "al-language-onprem/record/TestPublishedApplicationSysTable.al:294-365",
    ),
    (
        2_000_000_212,
        "1",
        "al-language-onprem/record/TestPublishedApplicationSysTable.al:275-291, :342-365",
    ),
    // Primary-key columns read by `Get`.
    (
        2_000_000_254,
        "5",
        "al-language/permissions/TestMetadataExpandedPermission.al:111, :124",
    ),
    (
        2_000_000_005,
        "4",
        "al-language/permissions/TestPermissionSetGrants.al:68",
    ),
    (
        2_000_000_139,
        "1",
        "al-language/handlers/TestReportMetadataColumns.al:49",
    ),
    (
        2_000_000_280,
        "1",
        "al-language/xmlport/TestXmlPortMetadataVirtualTable.al:42",
    ),
    (
        2_000_000_142,
        "1",
        "al-language/query/TestQueryMetadataVirtualTable.al:44",
    ),
    // Report Metadata, read by field name through a RecordRef (the helpers' FieldIndex loops).
    (
        2_000_000_139,
        "2",
        "al-language/report/lt-reportext-layout-relations/ReportMetadataRelations.al:151; al-language/misc/lt-probes-needed-3/PnC21Metadata.al:280",
    ),
    (
        2_000_000_139,
        "3",
        "al-language/report/lt-reportext-layout-relations/ReportMetadataRelations.al:152; al-language/metadata/lt-report-metadata-columns/ReportMetadata.al:202",
    ),
    // EnableExternalImages, the paper sources (Upper, Lower, Middle; another one poisons the row), FormatRegion
    // and ExcelLayoutMultipleDataSheets as declared; EnableExternalAssemblies No (unknown when declared).
    (
        2_000_000_139,
        "6",
        "al-language/report/lt-y-report-props/YHelper.al:226, YReportPropsTests.al:34-160",
    ),
    (
        2_000_000_139,
        "8",
        "al-language/report/lt-y-report-props/YHelper.al:226, YReportPropsTests.al:45",
    ),
    (
        2_000_000_139,
        "12",
        "al-language/report/lt-y-report-props/YHelper.al:226, YReportPropsTests.al:34-160",
    ),
    (
        2_000_000_139,
        "13",
        "al-language/report/lt-y-report-props/YHelper.al:226, YReportPropsTests.al:34-160",
    ),
    (
        2_000_000_139,
        "14",
        "al-language/report/lt-y-report-props/YHelper.al:226, YReportPropsTests.al:34-160",
    ),
    (
        2_000_000_139,
        "24",
        "al-language/report/lt-y-report-props/YHelper.al:226, YReportPropsTests.al:45, :64",
    ),
    (
        2_000_000_139,
        "25",
        "al-language/report/lt-y-report-props/YHelper.al:226, YReportPropsTests.al:34-160",
    ),
    // TransactionType UpdateNoLocks for a report that declares none; unknown for a declared one (al-vm reports.rs).
    (
        2_000_000_139,
        "11",
        "al-language/metadata/lt-report-metadata-columns/ReportMetadata.al:210, :236, :262",
    ),
    (
        2_000_000_139,
        "16",
        "al-language/report/lt-reportext-layout-relations/ReportExtensionLayoutRelations.al:82; al-language/report/lt-y-report-props/YReportParts.al:65-70",
    ),
    (
        2_000_000_139,
        "23",
        "al-language/metadata/lt-platform-metadata-columns/PlatformMetadataColumnValues.al:145",
    ),
    // BC 29 only: DefaultHeaderFooterPartName (30), DefaultThemePartName (31). By id, because the symbols of
    // older majors lack them.
    (
        2_000_000_139,
        "30",
        "al-language/report/lt-y-report-props/YReportParts.al:65-70, YReportPropsTests.al:45",
    ),
    (
        2_000_000_139,
        "31",
        "al-language/report/lt-y-report-props/YReportParts.al:65-70, YReportPropsTests.al:45",
    ),
    (
        2_000_000_136,
        "10",
        "al-language/metadata/lt-platform-metadata-columns/PlatformMetadataColumnValues.al:114",
    ),
    (
        2_000_000_063,
        "5",
        "al-language/metadata/lt-platform-metadata-columns/PlatformMetadataColumnValues.al:187, :210",
    ),
    // BC 29 only: "Layout Subtype" (19), HeaderFooterPart (20), ThemePart (21), read by field name, for layouts
    // that declare a Subtype; unknown for one that declares none (al-vm reports.rs). By id, as above.
    (
        2_000_000_234,
        "19",
        "al-language/report/lt-y-report-props/YReportParts.al:75-80",
    ),
    (
        2_000_000_234,
        "20",
        "al-language/report/lt-y-report-props/YReportParts.al:75-80",
    ),
    (
        2_000_000_234,
        "21",
        "al-language/report/lt-y-report-props/YReportParts.al:75-80",
    ),
    // XmlPort Metadata InherentPermissions and InherentEntitlements: blank or X as declared; another value is refused.
    (
        2_000_000_280,
        "InherentPermissions",
        "al-language/xmlport/lt-y-xmlport-props/YXmlPortProps.al:731-740",
    ),
    (
        2_000_000_280,
        "InherentEntitlements",
        "al-language/xmlport/lt-y-xmlport-props/YXmlPortProps.al:731-740",
    ),
    (
        2_000_000_140,
        "11",
        "al-language/events/lt-event-subscription-trigger-rows/EventSubscriptionTriggerRows.al:163",
    ),
];

/// Why column `field` of platform table `table` cannot be read, if it cannot.
pub fn unpinned_column(table: u32, field: u32) -> Option<String> {
    if field >= SYSTEM_OBJECT_BASE {
        return None;
    }
    let (_, pinned) = PINNED_COLUMNS.iter().find(|(t, _)| *t == table)?;
    if pinned.contains(&field) {
        return None;
    }
    let name = REGISTRY
        .iter()
        .chain(LATER_MAJORS)
        .find(|(i, _, _)| *i == table)
        .map_or("?", |(_, n, _)| *n);
    Some(format!(
        "column {field} of platform table '{name}' ({table}) is not pinned: no corpus test fixes its value (docs/probes-needed.md)"
    ))
}

/// The `Test ID` column of the per-test code coverage tables, as (table, field): Code Coverage Tests Run (2000000289)
/// and Code Coverage Test Lookup (2000000288).
pub const COVERAGE_TEST_ID: [(u32, u32); 2] = [(2_000_000_289, 1), (2_000_000_288, 4)];

/// Why `what` (a read of a coverage `Test ID`) is refused. The corpus pins only that the Test ID is the same for the tests
/// of a recording and for two recordings of one test, that the lookup rows of a test carry it, and that it is not 1
/// (coverage/W6CodeCoverageIds.al:111-112, :167-168; codeunit/TestCodeCoveragePerTest.al:116). BC computes the number per
/// run (codeunit/TestCodeCoveragePerTest.al:14), so any other read would show the runtime's own choice.
pub fn coverage_test_id_refusal(what: &str) -> String {
    format!(
        "{what}: the value of the Code Coverage Test ID is not pinned (only that it is equal across rows and recordings and \
         not 1, coverage/W6CodeCoverageIds.al:111-112, :167-168)"
    )
}

/// The first id of the platform's system objects.
pub const SYSTEM_OBJECT_BASE: u32 = 2_000_000_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_system_ids() {
        for (i, (id, name, _)) in REGISTRY.iter().enumerate() {
            assert!(*id >= SYSTEM_OBJECT_BASE, "{name} is not a system object");
            assert!(
                REGISTRY
                    .iter()
                    .skip(i + 1)
                    .all(|(j, n, _)| j != id && !n.eq_ignore_ascii_case(name)),
                "{name} is registered twice"
            );
        }
    }
}
