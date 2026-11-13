//! Platform option types whose values AL writes `FieldClass::Normal`: their members, in
//! ordinal order. Values are options at run time (`Value::Option`), so `Format` prints the
//! member name and `=` compares ordinals.
//!
//! `FieldClass` is the C/AL order. The `FieldType` member names are the platform's; their
//! ordinals are not pinned by the corpus (docs/probes-needed.md), so nothing here may be
//! relied on to print as a number.

pub fn members(name: &str) -> Option<&'static [&'static str]> {
    match name.to_ascii_lowercase().as_str() {
        "fieldclass" => Some(&["Normal", "FlowField", "FlowFilter"]),
        // SecurityFilter ordinals are measured on BC 27.5: Validated 0, Filtered 1, Ignored 2,
        // Disallowed 3, read with Format(x, 0, 2) and Evaluate from a number
        // (recordref-fieldref/recordref/TestPlatformOptionsLockingContracts.al:36-43).
        "securityfilter" => Some(&["Validated", "Filtered", "Ignored", "Disallowed"]),
        "isolationlevel" => Some(&[
            "Default",
            "ReadUncommitted",
            "ReadCommitted",
            "RepeatableRead",
            "UpdLock",
        ]),
        // The platform's object type option: AL names seven of these members (corpus
        // session-system/TestWebServiceActionContext.al:176-190 lists exactly Table, Page, Report,
        // Codeunit, XmlPort, Query and MenuSuite), the zero value `TableData` is not one of them
        // (the fresh value of `WebServiceActionContext.GetObjectType`). The ordinals are the C/AL
        // object type option's (docs/probes-needed.md); blank names are ordinals AL cannot name.
        "objecttype" => Some(&[
            "TableData",
            "Table",
            "",
            "Report",
            "",
            "Codeunit",
            "XmlPort",
            "MenuSuite",
            "Page",
            "Query",
        ]),
        // Five members over four values: Get and Updated are both 200
        // (session-system/TestWebServiceActionContext.al:100-110).
        // Declaration order of the platform's Verbosity: Critical first, so Error is not the zero
        // value (errors/errorinfo/TestErrorInfoType.al:30-34).
        "verbosity" => Some(&["Critical", "Error", "Warning", "Normal", "Verbose"]),
        // Microsoft docs, TelemetryScope: ExtensionPublisher, then All. Only passed to
        // Session.LogMessage, which changes nothing AL can see, so the ordinals are not pinned.
        "telemetryscope" => Some(&["ExtensionPublisher", "All"]),
        // Four members with the ordinals 1 to 4; a cleared value is 0, which has no name and formats as `0`
        // (format-evaluate/ACSecurityOperationResult.al:31-38 AC_SecurityOperationResult_Format, :62-68 _Variable). The blank
        // name is the ordinal AL cannot name.
        "securityoperationresult" => Some(&["", "Success", "ClientError", "Failure", "Timeout"]),
        // All 19 members with their ordinals, Other 0 to Euii 18
        // (format-evaluate/ACAuditCategory.al:83-101 AC_AuditCategory_FormatOrdinal; names :37-55).
        "auditcategory" => Some(&[
            "Other",
            "UserManagement",
            "GroupManagement",
            "Authentication",
            "Authorization",
            "RoleManagement",
            "ApplicationManagement",
            "KeyManagement",
            "DirectoryManagement",
            "ResourceManagement",
            "PolicyManagement",
            "DeviceManagement",
            "EntitlementManagement",
            "PasswordManagement",
            "IdentityProtection",
            "ObjectManagement",
            "ProvisioningManagement",
            "CustomerFacing",
            "Euii",
        ]),
        "dataclassification" => Some(&[
            "CustomerContent",
            "ToBeClassified",
            "EndUserIdentifiableInformation",
            "AccountData",
            "EndUserPseudonymousIdentifiers",
            "OrganizationIdentifiableInformation",
            "SystemMetadata",
            "AnonymousData",
        ]),
        // The platform's members and ordinals, probed (format-evaluate/W5TableConnections.al:`members`, `crm_format_2`,
        // `externalsql_format_2`): CRM 0, ExternalSQL 1, Exchange 2, MicrosoftGraph 3, Query 4.
        "tableconnectiontype" => Some(&["CRM", "ExternalSQL", "Exchange", "MicrosoftGraph", "Query"]),
        // The members in the order the documentation lists them (TransactionType option type); the
        // ordinals are not pinned and nothing may print them as numbers.
        "transactiontype" => Some(&["UpdateNoLocks", "Update", "Snapshot", "Browse", "Report"]),
        // The report layout kinds, in the documentation's order. A documentation-only assumption: no corpus
        // test names the members or pins an ordinal, so nothing may print the number (docs/probes-needed.md,
        // "ReportLayoutType"). Base Application compares it with `ReportLayoutType::Word` and `::RDLC`.
        "reportlayouttype" => Some(&["RDLC", "Word", "Excel", "Custom"]),
        // The platform's ordinals (text/PnBReportSaveAs.al:392-396 OtherFormats_NoLayout): Excel 1, Pdf 2,
        // Word 3, Xml 4, Html 5. Ordinal 0 has no name AL can use.
        "reportformat" => Some(&["", "Excel", "Pdf", "Word", "Xml", "Html"]),
        "errortype" => Some(&["Client", "Internal"]),
        // The members in the documentation's order (HttpRequestType option type). A handler formats the value
        // (`Get`, `Head`: session-system/PZHttpHandler.al on the staging branch); the ordinals are not pinned and nothing may
        // print them.
        "httprequesttype" => Some(&[
            "Unknown", "Get", "Delete", "Post", "Put", "Patch", "Options", "Head", "Connect", "Trace",
        ]),
        // The platform's ordinals, probed on BC 27.5 and 29.0 (types/PWMisc.al:ClientType_OrdinalsAndNames, evaluated from
        // 0 to 30; nothing above 15): Current 0, Default 1, Windows 2, Web 3, SOAP 4, OData 5, NAS 6, Background 7,
        // Management 8, Tablet 9, Phone 10, Desktop 11, Teams 12, ODataV4 13, Api 14, ChildSession 15.
        "clienttype" => Some(&[
            "Current",
            "Default",
            "Windows",
            "Web",
            "SOAP",
            "OData",
            "NAS",
            "Background",
            "Management",
            "Tablet",
            "Phone",
            "Desktop",
            "Teams",
            "ODataV4",
            "Api",
            "ChildSession",
        ]),
        // The report's default layout and a test codeunit's TestPermissions at their platform ordinals: None 0, RDLC 1, Word 2,
        // Excel 3; InheritFromTestCodeunit 0, Restrictive 1, NonRestrictive 2, Disabled 3 (format-evaluate/
        // SoEvaluateSystemOptions.al:1888 and :2154, Names and Numbers). Report.DefaultLayout(Number) formats RDLC, Word, Excel
        // and None (metadata/platform-tables/ReportMetadata.al:272, reports/runtime/YReportPropsTests.al:236); Custom is a
        // member of the Report Metadata column that Evaluate does not read, its ordinal is not pinned.
        "defaultlayout" => Some(&["None", "RDLC", "Word", "Excel", "Custom"]),
        "testpermissions" => Some(&["InheritFromTestCodeunit", "Restrictive", "NonRestrictive", "Disabled"]),
        "datascope" => Some(&["Module", "Company", "User", "CompanyAndUser"]),
        // Normal and Install are pinned (install-upgrade/TestInstallEnvVisible_Tests.al:90-127). The ordinals are Normal 0,
        // Install 1, Uninstall 2, Upgrade 3 (format-evaluate/SoEvaluateSystemOptions.al:2678); the blank name is Uninstall,
        // which the runtime does not let AL name (`system_option_eval::hidden_name`).
        "executioncontext" => Some(&["Normal", "Install", "", "Upgrade"]),
        // Standard and Debug only (corpus must-not-compile 0010 to 0013 and 0019: Install, Uninstall, Upgrade,
        // Report and Profile do not exist). A test run formats as `Standard`
        // (session-system/TestEnvironmentUserTenantContracts.al:247); the ordinals are not pinned.
        "executionmode" => Some(&["Standard", "Debug"]),
        // Only `LocalScope` is named by the corpus; the ordinals are not pinned.
        "notificationscope" => Some(&["LocalScope", "GlobalScope"]),
        "webserviceactionresultcode" => Some(&["None", "Get", "Created", "Updated", "Deleted"]),
        // The StyleExpr member type. The ordinals (None 1 to Subordinate 11) and the value 0 of a variable never assigned or cleared,
        // which has no member and formats as `0`, are read with Format(x, 0, 2) and Format(x) (types/page-style/PageStyle.al:
        // PageStyle_FormatNumber2_AllMembers, PageStyle_Variable_Clear). The empty name at 0 formats as its number.
        "pagestyle" => Some(&[
            "",
            "None",
            "Standard",
            "StandardAccent",
            "Strong",
            "StrongAccent",
            "Attention",
            "AttentionAccent",
            "Favorable",
            "Unfavorable",
            "Ambiguous",
            "Subordinate",
        ]),
        // The ErrorLevel of `CurrPage.EnqueueBackgroundTask`. Error is the zero value; the platform's ordinals are 0, 10 and 20
        // (`platform_ordinal`), and Format(x, 0, 1) prints the name (format-evaluate/ACPageBackgroundTaskErrorLevel.al).
        "pagebackgroundtaskerrorlevel" => Some(&["Error", "Warning", "Ignore"]),
        "fieldtype" => Some(&[
            "TableFilter",
            "RecordID",
            "OemText",
            "Date",
            "Time",
            "DateFormula",
            "Decimal",
            "Media",
            "MediaSet",
            "Text",
            "Code",
            "Binary",
            "Boolean",
            "Integer",
            "OemCode",
            "Option",
            "BigInteger",
            "BLOB",
            "GUID",
            "DateTime",
            "Duration",
        ]),
        _ => None,
    }
}

/// Whether `members` is the `IsolationLevel` option.
pub fn is_isolation_level(members: &[String]) -> bool {
    let known = members_static("isolationlevel");
    members.len() == known.len() && members.iter().zip(known).all(|(a, b)| a == b)
}

/// Whether `members` is the `ClientType` option: an ordinal or a member name evaluates into it, any other text does not.
pub fn is_client_type(members: &[String]) -> bool {
    let known = members_static("clienttype");
    members.len() == known.len() && members.iter().zip(known).all(|(a, b)| a == b)
}

/// For the platform options whose `Evaluate` of a number past the members is measured to answer false, the largest number
/// measured: `TableConnectionType` 10, `ExecutionMode` 12 (see `evaluate.rs`).
pub fn rejected_number_past_members(members: &[String]) -> Option<usize> {
    [("tableconnectiontype", 10), ("executionmode", 12)]
        .into_iter()
        .find_map(|(name, last)| {
            let known = members_static(name);
            (members.len() == known.len() && members.iter().zip(known).all(|(a, b)| a == b)).then_some(last)
        })
}

/// For `AuditCategory` and `SecurityOperationResult`, the member names in the order the error of a failed `Evaluate` lists
/// them, which is also the order whose first member the empty text evaluates to: AuditCategory in ordinal order
/// (format-evaluate/ACAuditCategory.al:486-488), SecurityOperationResult as `Success, Failure, ClientError, Timeout`
/// (format-evaluate/ACSecurityOperationResult.al:206-208, where the empty text gives ordinal 1).
pub fn audit_option_listing(members: &[String]) -> Option<&'static [&'static str]> {
    let same = |name: &str| {
        let known = members_static(name);
        members.len() == known.len() && members.iter().zip(known).all(|(a, b)| a == b)
    };
    if same("auditcategory") {
        return Some(members_static("auditcategory"));
    }
    if same("securityoperationresult") {
        return Some(&["Success", "Failure", "ClientError", "Timeout"]);
    }
    None
}

/// Whether `members` are those of the Field table's `Type` option, whose ordinals are not its positions.
pub fn is_field_table_type(members: &[String]) -> bool {
    const MEMBERS: [&str; 21] = [
        "TableFilter",
        "RecordID",
        "OemText",
        "Date",
        "Time",
        "DateFormula",
        "Decimal",
        "Media",
        "MediaSet",
        "Text",
        "Code",
        "Binary",
        "BLOB",
        "Boolean",
        "Integer",
        "OemCode",
        "Option",
        "BigInteger",
        "Duration",
        "GUID",
        "DateTime",
    ];
    members.len() == MEMBERS.len() && members.iter().zip(MEMBERS).all(|(a, b)| a == b)
}

/// Whether `members` is the `FieldType` option. AL cannot read its ordinals, so `Format(x, 0, 2)`
/// prints the member name like any other format (recordref-fieldref/recordref/TestPlatformOptionsLockingContracts.al:
/// 76, "format 2 prints the name too, not an ordinal").
pub fn ordinals_hidden(members: &[String], standard: i32) -> bool {
    let known = members_static("fieldtype");
    let same = |known: &[&str]| members.len() == known.len() && members.iter().zip(known).all(|(a, b)| a == b);
    // The Action option too: `Format(Action::OK, 0, 2)` is `OK` (ui-handlers/PnGActionOrdinals.al:MemberOrdinals). A FieldClass
    // value shows its name in numbers 2 and 9 (format-evaluate/SoEvaluateSystemOptions.al:3769 FormatOfVariable_Members
    // `Normal|Normal|Normal|Normal|Normal`). The Field table's Class column has the same members but shows its number: the
    // lowering sends it to `System.FormatOptionColumn#1` (:3826, metadata/platform-tables/R6890SystemFieldClassification.al:103).
    same(known)
        || same(&crate::value::ACTION_MEMBERS)
        || (matches!(standard, 2 | 9) && same(members_static("fieldclass")))
}

/// Whether `members` is the `FieldClass` option (also the members of the Field table's Class column).
pub fn is_field_class(members: &[String]) -> bool {
    let known = members_static("fieldclass");
    members.len() == known.len() && members.iter().zip(known).all(|(a, b)| a == b)
}

/// Whether `members` is the `PageBackgroundTaskErrorLevel` option.
pub fn is_page_background_task_error_level(members: &[String]) -> bool {
    let known = members_static("pagebackgroundtaskerrorlevel");
    members.len() == known.len() && members.iter().zip(known).all(|(a, b)| a == b)
}

/// Whether `members` is the `SecurityFilter` option, the one platform option an ordinal can be
/// evaluated into (recordref-fieldref/recordref/TestPlatformOptionsLockingContracts.al:42).
pub fn is_security_filter(members: &[String]) -> bool {
    let known = members_static("securityfilter");
    members.len() == known.len() && members.iter().zip(known).all(|(a, b)| a == b)
}

fn members_static(name: &str) -> &'static [&'static str] {
    members(name).expect("known")
}

/// The member names as an owned option type.
pub fn option_members(name: &str) -> Option<Vec<String>> {
    members(name).map(|m| m.iter().map(|s| s.to_string()).collect())
}

/// The member list a value whose declared type is TransactionType carries in a Variant (`Variant.FromTransactionType`, which
/// al-sema writes where such a value goes into a Variant): one shared list, so [`is_transaction_type`] tells it by identity from
/// an Option with the same members (an app Option, the TransactionType field of "XmlPort Metadata").
pub fn transaction_type_members() -> std::sync::Arc<[String]> {
    static LIST: std::sync::OnceLock<std::sync::Arc<[String]>> = std::sync::OnceLock::new();
    LIST.get_or_init(|| option_members("transactiontype").expect("known").into())
        .clone()
}

/// Whether an option value with these members is a TransactionType value in a Variant (see [`transaction_type_members`]).
pub fn is_transaction_type(members: &std::sync::Arc<[String]>) -> bool {
    std::sync::Arc::ptr_eq(members, &transaction_type_members())
}

/// The platform enum a value of option type with these members belongs to, when it is one the `Is<Type>` tests of a Variant
/// tell apart (types/variant/LTP89IsEnumsA.al and LTP89IsEnumsB.al: such a value is `IsAction`, say, and
/// not `IsOption`). The members decide, so a user option with exactly these members counts as the platform one.
pub fn kind_of(members: &[String]) -> Option<&'static str> {
    let same = |known: &[&str]| members.len() == known.len() && members.iter().zip(known).all(|(a, b)| a == b);
    if same(&crate::value::ACTION_MEMBERS) {
        return Some("action");
    }
    if same(&crate::value::PROMPT_MODE_MEMBERS) {
        return Some("promptmode");
    }
    if same(&["MSDos", "UTF8", "UTF16", "Windows"]) {
        return Some("textencoding");
    }
    [
        "clienttype",
        "dataclassification",
        "defaultlayout",
        "executionmode",
        "objecttype",
        "reportformat",
        "tableconnectiontype",
        "testpermissions",
    ]
    .into_iter()
    .find(|name| same(members_static(name)))
}

/// Whether `members` belong to a platform option or enum this module lists (also the ones `kind_of` does not tell apart), as opposed to an
/// option a variable or a table field of the app declares.
pub fn is_platform(members: &[String]) -> bool {
    const NAMES: &[&str] = &[
        "fieldclass",
        "securityfilter",
        "isolationlevel",
        "objecttype",
        "verbosity",
        "telemetryscope",
        "securityoperationresult",
        "auditcategory",
        "dataclassification",
        "tableconnectiontype",
        "transactiontype",
        "reportlayouttype",
        "reportformat",
        "errortype",
        "httprequesttype",
        "clienttype",
        "defaultlayout",
        "testpermissions",
        "datascope",
        "executioncontext",
        "executionmode",
        "notificationscope",
        "webserviceactionresultcode",
        "fieldtype",
        "pagestyle",
        "pagebackgroundtaskerrorlevel",
    ];
    kind_of(members).is_some()
        || is_field_table_type(members)
        || NAMES.iter().any(|name| {
            members.len() == members_static(name).len() && members.iter().zip(members_static(name)).all(|(a, b)| a == b)
        })
}

/// The ordinal the platform gives member `position` of a platform option whose ordinals are not its positions, shown by
/// `Format(x, 0, 2)`: Verbosity and TelemetryScope count from 1, the web service result codes are the HTTP codes
/// (errors/PnBErrorsCollections.al:VerbosityAndDataClassification_Ordinals,
/// WebServiceActionContext_Ordinals_AndDuplicateKey; session/TestLogMessage... `enum_ints`).
pub fn platform_ordinal(members: &[String], position: i32) -> Option<i32> {
    let same = |name: &str| {
        let known = members_static(name);
        members.len() == known.len() && members.iter().zip(known).all(|(a, b)| a == b)
    };
    if same("verbosity") || same("telemetryscope") {
        return Some(position + 1);
    }
    // Error 0, Warning 10, Ignore 20 (format-evaluate/ACPageBackgroundTaskErrorLevel.al:AC_PageBackgroundTaskErrorLevel_Format `ordinal_*`).
    if same("pagebackgroundtaskerrorlevel") {
        return Some(position * 10);
    }
    if same("webserviceactionresultcode") {
        return [0, 200, 201, 200, 204].get(usize::try_from(position).ok()?).copied();
    }
    None
}
