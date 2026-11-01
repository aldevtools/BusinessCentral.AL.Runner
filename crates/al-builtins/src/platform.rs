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
    /// No rows: an OnPrem tenant is not set up for the cloud (al-language-onprem/platform/PtOnPremTables.al:248).
    IntelligentCloud,
    /// Media live in the session's media store (ADR-017); the table is not a view of it yet.
    TenantMedia,
    /// Media (OnPrem): a read keyed to the null ID finds nothing (the Logo of an app without one, onprem
    /// session/PZPublishedApplicationColumns.al Logo_*, corpus branch al-runtime-rs-staging-ug); any other read is refused.
    Media,
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
    /// One row per object; only a query's Metadata Blob is pinned, as the input of Microsoft's `QueryMetadataReader`
    /// (al-vm `platform/objects.rs`, `application_object_metadata_rows`).
    ApplicationObjectMetadata,
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
    (
        2_000_000_207,
        "Application Object Metadata",
        Provider::Virtual(Virtual::ApplicationObjectMetadata),
    ),
    (2_000_000_049, "Code Coverage", Provider::Stored),
    (2_000_000_288, "Code Coverage Test Lookup", Provider::Stored),
    (2_000_000_289, "Code Coverage Tests Run", Provider::Stored),
    (2_000_000_184, "Tenant Media", Provider::Virtual(Virtual::TenantMedia)),
    (2_000_000_181, "Media", Provider::Virtual(Virtual::Media)),
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
    // Read from an OnPrem app: IsEmpty Yes, Count 0, Get and FindFirst No, inside a TryFunction too without an error
    // (al-language-onprem/platform/PtOnPremTables.al:248, QZ_IntelligentCloud_Read). Writes are not pinned: a virtual
    // table refuses them.
    (
        2_000_000_146,
        "Intelligent Cloud",
        Provider::Virtual(Virtual::IntelligentCloud),
    ),
    // Ordinary tables that a fresh tenant fills with pinned rows (al-vm `platform/seed.rs`): Insert(true) of an existing key
    // answers false and Modify(true) true (platform/lt-platform-tables/PtCloudTables.al:378-432 and :435-492).
    (2_000_000_233, "Tenant Report Layout Selection", Provider::Stored),
    // A fresh tenant holds rows here that nothing pins (SaaS: 12), none of a report of the app under test; an Insert of a
    // row whose Layout is the report's own RDLC layout is stored as given (report/lt-sc-report-metadata-views/
    // Y6TenantReportLayoutRows.al, corpus master). What else is pinned: al-vm `tenant_layouts.rs`.
    (2_000_000_232, "Tenant Report Layout", Provider::Stored),
    (2_000_000_179, "OData Edm Type", Provider::Stored),
    // Empty on a fresh tenant; Insert, Modify, Delete and DeleteAll without triggers work (al-language-onprem/session/
    // PZPlatformSurvey.al:142-145, :166-169; records/W4PlatformAndMeta.al:123). Document Service's own insert trigger is
    // native (al-vm recmeta.rs `document_service_insert`).
    (2_000_000_255, "Signup Context", Provider::Stored),
    (2_000_000_114, "Document Service", Provider::Stored),
    // An ordinary table, empty on a fresh tenant (al-language-onprem/platform/PtOnPremTables.al:214-236, session/
    // PZPlatformSurvey.al:172-175); its rows follow the rename of their tenant profile (al-vm profiles.rs).
    (2_000_000_187, "Tenant Profile Page Metadata", Provider::Stored),
    // OnPrem tables that a fresh tenant leaves empty and that take Insert, a duplicate Insert (false), Modify, Delete, an Insert
    // without triggers and DeleteAll (al-language-onprem/platform/PtOnPremTables.al:57-78, :80-101, :149-170, :192-213).
    // Tenant Profile reads "Customization Status" Updated after an Insert (:200), its first option and so its default.
    (2_000_000_170, "Configuration Package File", Provider::Stored),
    (2_000_000_115, "Document Service Scenario", Provider::Stored),
    (2_000_000_228, "Report Settings Override", Provider::Stored),
    (2_000_000_177, "Tenant Profile", Provider::Stored),
    // Empty on a fresh tenant; an Insert raises the license error and stores nothing (PtOnPremTables.al:172-190). Only that
    // Insert, with triggers, is pinned: the others refuse (al-vm records.rs).
    (2_000_000_130, "Device", Provider::Stored),
    // Empty on the corpus containers, and ordinary stored tables (corpus master platform/lt-y35-stored-tables/
    // Y35StoredTables.al, onprem): Webhook Subscription and Webhook Notification take Insert, a duplicate Insert (false),
    // filters, Delete(true) and DeleteAll, and keep the rows as written, except that the platform fills a blank
    // "Application ID" of a subscription with 'NAV' (al-vm records.rs `webhook_application_id`). MCP Configuration Tool
    // takes Insert, Get, GetBySystemId, Modify, ModifyAll and Delete without a parent or object check; a Delete of an MCP
    // Configuration with tools differs by major and is refused (al-vm records.rs `stored_platform_check`).
    // Privacy Notice Approval (Cloud, main suite:
    // platform/lt-y35-privacy-approval/Y35PrivacyApproval.al) is empty, takes Insert, Modify and DeleteAll, leaves
    // "Approver User SID" as written and fires its table events; Privacy Notice's FlowFields read it per exact User SID.
    (2_000_000_199, "Webhook Subscription", Provider::Stored),
    (2_000_000_194, "Webhook Notification", Provider::Stored),
    (2_000_000_293, "MCP Configuration Tool", Provider::Stored),
    (2_000_000_238, "Privacy Notice Approval", Provider::Stored),
    // OnPrem; takes Insert, a duplicate Insert (false), Modify, Delete, an Insert without triggers and DeleteAll, and keeps
    // the Blob File it is given (al-language-onprem/platform/PtOnPremTables.al Y17_PowerBIBlob_InsertModifyDelete,
    // Y17_PowerBIBlob_BlobRoundTrip, corpus master 97c29f86). A fresh tenant holds demo rows nothing pins: `UNSEEDED_ROWS`.
    (2_000_000_144, "Power BI Blob", Provider::Stored),
    // OnPrem; a plain stored table: Insert (a blank Id and Context too), a duplicate Insert (false), Get, SetRange, Modify,
    // Rename, Delete and DeleteAll, no validation (al-language-onprem/platform/PtOnPremTables.al:365, :395, corpus master
    // ac3499d6). A fresh tenant holds demo rows nothing pins: `UNSEEDED_ROWS`.
    (2_000_000_145, "Power BI Default Selection", Provider::Stored),
];

/// Stored tables that a fresh tenant fills with rows no corpus test pins. The runtime starts them empty, so it treats them as
/// unseeded (`Engine::unseeded`): writes work, and a read of rows that were not written, or a `Get` that finds nothing, is
/// refused. Web Service and Tenant Web Service are seeded with their pinned rows instead (al-vm `platform/seed.rs`;
/// PtWebServiceRows.al:39-73, PlatformRowsWebService.al:66-215). Their System Application neighbours are not: Tenant Web
/// Service OData (6710) holds 29 rows and Tenant Web Service Columns (6711) 402 on a fresh tenant, only the counts are pinned
/// (onprem platform/PtWebServiceRows.al:237, :267 'counts before'); what Tenant Web Service Filter (6712) holds is not pinned.
/// Power BI Blob (2000000144) holds demo rows on a fresh tenant (8 on the Linux CI containers, recorded and not asserted:
/// PtOnPremTables.al Y17_PowerBIBlob_BlobRoundTrip); once the session's own DeleteAll without filters has emptied it, it is
/// answered (al-vm `Session::is_unseeded`). Power BI Default Selection (2000000145) likewise (8 demo rows on the Linux CI
/// containers, Ignored: PtOnPremTables.al:365, corpus master ac3499d6).
pub const UNSEEDED_ROWS: &[u32] = &[6710, 6711, 6712, 2_000_000_144, 2_000_000_145];

/// Platform tables that only the System app of a later major declares (the registry above is checked against the System app
/// of the first major the corpus pins, `tests/platform_tables.rs`). Ordinary stored tables here, written by install code:
/// "Agent Install" (System Application "Agent Upgrade") and "MCP Install" insert into them, and without a provider the install
/// baseline of those majors stopped at them and left tables unseeded. A Cloud-target `RecordRef.Open` of Agent Creation Control
/// is refused (platform/lt-aw-fresh-tenant/AwTests.al:590-616, `al-vm` refs.rs).
pub const LATER_MAJORS: &[(u32, &str, Provider)] = &[
    (2_000_000_297, "Agent Creation Control", Provider::Stored),
    (2_000_000_292, "MCP Configuration", Provider::Stored),
    // BC 29 on. Empty on a fresh tenant and writable; an Insert checks the layout parts it names (al-vm
    // tenant_layout_cfg.rs; platform/lt-platform-tables/PtCloudTables.al Y17_TenantReportLayoutCfg_*, corpus master 97c29f86).
    (2_000_000_249, "Tenant Report Layout Cfg", Provider::Stored),
    // BC 29 on. Empty on a fresh tenant; Insert, a duplicate Insert (false), Modify, Rename, Delete and DeleteAll, with no
    // validation of the report or the company (platform/lt-platform-tables/PtCloudTables.al:794, :824, corpus master
    // ac3499d6). Report Layout List shows its overrides (al-vm platform/reports.rs `apply_layout_overrides`, :901).
    (2_000_000_248, "Tenant Report Layout Override", Provider::Stored),
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
    2_000_000_207,
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
    // Application Object Metadata. Metadata of a query: present, and what QueryMetadataReader.FromStream reads from it
    // (System Application through Base Application's ODataUtility.GetTenantWebServiceMetadata). Its bytes stay opaque.
    (
        2_000_000_207,
        "6",
        "al-language-onprem-dotnet/lt-qx-dotnet/QxDotNetDemoShapes.al:173-181; lt-re-dotnet/ReQueryMetadataShapes.al:116-140",
    ),
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
    // EnableHyperLinks, ShowPrintStatus and PreviewMode of a report that declares none of them, read through the helper's
    // FieldIndex loop; unknown for a declared one (al-vm reports.rs).
    (
        2_000_000_139,
        "7",
        "al-language/metadata/lt-report-metadata-columns/ReportMetadata.al:22-31, :206",
    ),
    (
        2_000_000_139,
        "10",
        "al-language/metadata/lt-report-metadata-columns/ReportMetadata.al:22-31, :209",
    ),
    (
        2_000_000_139,
        "15",
        "al-language/metadata/lt-report-metadata-columns/ReportMetadata.al:22-31, :214",
    ),
    // UseSystemPrinter as declared, No for a report that does not declare it (al-vm reports.rs).
    (
        2_000_000_139,
        "5",
        "al-language/misc/lt-y13-triage/Y13ReportSystemPrinter.al:526, :530-543; al-language/metadata/lt-report-metadata-columns/ReportMetadata.al:204",
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
    // Table Relations Metadata names of the own and the related table and field, read through the helper's FieldIndex loop
    // (al-vm metadata.rs).
    (
        2_000_000_141,
        "12",
        "al-language/misc/lt-probes-needed-5/PnE09Platform.al:82-93, :122-125",
    ),
    (
        2_000_000_141,
        "13",
        "al-language/misc/lt-probes-needed-5/PnE09Platform.al:82-93, :122-125",
    ),
    (
        2_000_000_141,
        "14",
        "al-language/misc/lt-probes-needed-5/PnE09Platform.al:82-93, :122-125",
    ),
    (
        2_000_000_141,
        "15",
        "al-language/misc/lt-probes-needed-5/PnE09Platform.al:82-93, :122-125",
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
    // Table Metadata InherentPermissions: `X` or blank as declared; another value is unknown (al-vm objects.rs).
    (
        2_000_000_136,
        "19",
        "al-language/permissions/W6PermissionColumns.al Y17_TableMetadata_InherentPermissions (corpus master 97c29f86)",
    ),
    // Metadata Permission "Security Filter": blank for every row of an app permission set (a permissionset object has no
    // syntax for one), plain and with format 9.
    (
        2_000_000_251,
        "14",
        "al-language/permissions/W6PermissionColumns.al Y17_MetadataPermission_SecurityFilter_OwnSet, _AllSets (corpus master 97c29f86)",
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
    // Report Layout List of a layout declared in AL, read through a RecordRef (al-vm reports.rs): CaptionML (4) blank for a
    // layout that declares none, Runtime Package ID (5) a non-null guid other than the app id, Custom Type (8) the null guid
    // and MIME Type (9) blank for an RDLC, Word or Excel layout, Report Name (12) the name of a
    // report that declares no caption, Layout Publisher (13) "<app> by <publisher>", User Defined (14) No for an RDLC, Word or
    // Excel layout, ReportIsInstalled (15) Yes, Layout Status (18, BC 28+) Approved for a layout without a Subtype
    // (platform/lt-ug-report-layout-list/UgReportLayoutList.al:94-150; platform/lt-aw-fresh-tenant/AwTests.al:220-300).
    (
        2_000_000_234,
        "4",
        "al-language/platform/lt-ug-report-layout-list/UgReportLayoutList.al:103, :137",
    ),
    (
        2_000_000_234,
        "5",
        "al-language/platform/lt-ug-report-layout-list/UgReportLayoutList.al:101, :135",
    ),
    (
        2_000_000_234,
        "8",
        "al-language/platform/lt-ug-report-layout-list/UgReportLayoutList.al:105, :139; platform/lt-aw-fresh-tenant/AwTests.al:228",
    ),
    (
        2_000_000_234,
        "9",
        "al-language/platform/lt-ug-report-layout-list/UgReportLayoutList.al:106, :140; platform/lt-aw-fresh-tenant/AwTests.al:228",
    ),
    (
        2_000_000_234,
        "12",
        "al-language/platform/lt-ug-report-layout-list/UgReportLayoutList.al:110, :144; platform/lt-aw-fresh-tenant/AwTests.al:228",
    ),
    (
        2_000_000_234,
        "13",
        "al-language/platform/lt-ug-report-layout-list/UgReportLayoutList.al:111, :145; platform/lt-aw-fresh-tenant/AwTests.al:228",
    ),
    (
        2_000_000_234,
        "14",
        "al-language/platform/lt-ug-report-layout-list/UgReportLayoutList.al:112, :146; platform/lt-aw-fresh-tenant/AwTests.al:228",
    ),
    (
        2_000_000_234,
        "15",
        "al-language/platform/lt-ug-report-layout-list/UgReportLayoutList.al:113, :147; platform/lt-aw-fresh-tenant/AwTests.al:228",
    ),
    (
        2_000_000_234,
        "18",
        "al-language/platform/lt-ug-report-layout-list/UgReportLayoutList.al:117, :151; platform/lt-aw-fresh-tenant/AwTests.al:228",
    ),
    // BC 29 only: "Layout Subtype" (19), HeaderFooterPart (20), ThemePart (21), read by field name, for layouts that declare
    // a Subtype, and through a RecordRef as Default and blank for one that declares none (UgReportLayoutList.al:119-121). By id.
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
    // Event Subscription "Originating Package ID": a Guid, scrubbed by the observer because it changes per publish; read through
    // the helper's FieldIndex loop (al-vm metadata.rs).
    (
        2_000_000_140,
        "10",
        "al-language/misc/lt-probes-needed-5/PnE11EventSub.al:67-78, :99",
    ),
    (
        2_000_000_140,
        "11",
        "al-language/events/lt-event-subscription-trigger-rows/EventSubscriptionTriggerRows.al:163",
    ),
];

/// Columns pinned on a corpus writer branch whose bump into the pinned corpus is pending (a runtime fix for a user issue
/// before its pin lands, docs/agents/brief.md): they read as pinned ones, and a read lowered from AL code marks the
/// assumption (`assumption_of_column`). When the bump lands, each moves to `PINNED_ELSEWHERE` (or the generated list) and
/// its assumption is deleted. As (table id, field id, assumption).
pub const AWAITING_COLUMNS: &[(u32, u32, crate::assumptions::Assumption)] = &[
    // Published Application Logo: blank for an app whose manifest declares none, the logo file for a PNG logo, unknown for
    // any other app (al-vm apps.rs).
    (
        2_000_000_206,
        20,
        crate::assumptions::Assumption::PublishedApplicationLogoIsTheLogoFile,
    ),
];

/// The assumption a read of column `field` of platform table `table` rests on (`AWAITING_COLUMNS`).
pub fn assumption_of_column(table: u32, field: u32) -> Option<crate::assumptions::Assumption> {
    AWAITING_COLUMNS
        .iter()
        .find(|(t, f, _)| *t == table && *f == field)
        .map(|(_, _, a)| *a)
}

/// Why column `field` of platform table `table` cannot be read, if it cannot.
pub fn unpinned_column(table: u32, field: u32) -> Option<String> {
    if field >= SYSTEM_OBJECT_BASE || assumption_of_column(table, field).is_some() {
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
