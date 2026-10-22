//! The AL result types of the `DotNet` members the runtime provides itself (ADR-014 section 1,
//! the registry in `al-vm/src/dotnet/`).
//!
//! A member of a .NET type is normally untyped at compile time: BC types it from the assembly,
//! which lowering never opens. A registry member is different. The runtime implements it, so it
//! knows what the member returns, and a result of a known scalar type is an ordinary operand
//! (`if Helper.IsAzure() then`, `Settings.ApiSubscriptionDelayTime() > 0`).
//!
//! This table is read by `al-sema` (typing) and checked against the registry by a test in `al-vm`
//! (every registry member has a row here and every row has a member), so the two cannot drift.
//! Types and members compare ignoring ASCII case, as AL does.

/// What a registry member returns, as far as AL's type checker is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ret {
    Boolean,
    Integer,
    Text,
    /// A `DotNet` object (for example `Instance()`); its type stays "known at run time".
    Object,
    /// No result (a setter): the call is a statement.
    Void,
}

/// `Microsoft.Dynamics.Nav.Types.ALConfigSettings`.
pub const AL_CONFIG_SETTINGS: &str = "Microsoft.Dynamics.Nav.Types.ALConfigSettings";
/// `Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper`.
pub const TENANT_SETTINGS_HELPER: &str = "Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper";
/// `Microsoft.Dynamics.Nav.Runtime.NavTestExecution`.
pub const TEST_EXECUTION: &str = "Microsoft.Dynamics.Nav.Runtime.NavTestExecution";
/// `Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper`, which codeunit 131006 "Permissions Mock" constructs.
pub const PERMISSION_TEST_HELPER: &str = "Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper";
/// `Microsoft.Dynamics.Nav.NavUserAccount.NavUserAccountHelper`.
/// Azure AD Graph's query object (System Application "Azure AD Graph Impl.").
pub const GRAPH_QUERY: &str = "Microsoft.Dynamics.Nav.AzureADGraphClient.GraphQuery";
/// `Microsoft.Dynamics.Nav.LicensingService.Model.TenantInfo` (the answer of `GraphQuery.GetTenantDetail`).
pub const TENANT_INFO: &str = "Microsoft.Dynamics.Nav.LicensingService.Model.TenantInfo";
pub const USER_ACCOUNT_HELPER: &str = "Microsoft.Dynamics.Nav.NavUserAccount.NavUserAccountHelper";

/// `System.IO.Compression.GZipStream`, behind System Application's "Data Compression".
/// The exceptions `GetLastErrorObject` answers and the `System.Type` of `GetType()` on them
/// (tests/al-language-onprem-dotnet/lt-dotnet-interop/GetLastErrorObject.al).
pub const NCL_DIALOG_EXCEPTION: &str = "Microsoft.Dynamics.Nav.Types.Exceptions.NavNCLDialogException";
pub const RECORD_NOT_FOUND_EXCEPTION: &str = "Microsoft.Dynamics.Nav.Types.Exceptions.NavCSideRecordNotFoundException";
pub const AL_EXCEPTION: &str = "Microsoft.Dynamics.Nav.Types.Exceptions.NavALException";
/// A failing `TestField`, an `Evaluate` without a return value, and the base exception of a division by zero
/// (tests/al-language-onprem-dotnet/lt-dotnet-interop/GetLastErrorObjectKinds.al).
pub const TEST_FIELD_EXCEPTION: &str = "Microsoft.Dynamics.Nav.Types.NavTestFieldException";
pub const NCL_EVALUATE_EXCEPTION: &str = "Microsoft.Dynamics.Nav.Types.Exceptions.NavNCLEvaluateException";
pub const DIVIDE_BY_ZERO_EXCEPTION: &str = "System.DivideByZeroException";
/// The inner exception of a `NavALException`: only that it is not null is pinned, so it has no member.
pub const AL_INNER_EXCEPTION: &str = "<inner exception of NavALException (type not pinned)>";
/// What `GetLastErrorObject` answers when there is no last error (after `ClearLastError`): `IsNull` is true and
/// `Format` is 'null' (GetLastErrorObject.al:295-298), but a member raises the not-instantiated error, as for an
/// unassigned variable. It is not `Value::Null`, because `Format` of an unassigned variable is not 'null'
/// (TestDotNetValueKindsContracts.al:15).
pub const NULL_EXCEPTION: &str = "<no last error>";
/// What `GetType()` of a last-error exception answers. The name must not be a real CLR type name: the host
/// reports `System.RuntimeType` for the `System.Type` objects it holds (GetDotNetType), and a registry type of
/// that name would take their members.
pub const RUNTIME_TYPE: &str = "<Type of a last-error exception>";

/// The `System.Drawing` types `O365 Brand Color`.MakePicture uses (ADR-044).
pub const BITMAP: &str = "System.Drawing.Bitmap";
pub const GRAPHICS: &str = "System.Drawing.Graphics";
pub const COLOR_TRANSLATOR: &str = "System.Drawing.ColorTranslator";
pub const COLOR: &str = "System.Drawing.Color";
pub const SOLID_BRUSH: &str = "System.Drawing.SolidBrush";
pub const IMAGE_FORMAT: &str = "System.Drawing.Imaging.ImageFormat";

pub const GZIP_STREAM: &str = "System.IO.Compression.GZipStream";
/// `System.IO.Compression.CompressionMode`.
pub const COMPRESSION_MODE: &str = "System.IO.Compression.CompressionMode";

/// (CLR type, member, result). One row per registry member that returns a value.
/// `System.IO.DirectoryInfo`, served only for a folder AL Runner staged for the company build (al-vm dotnet/staged_dir.rs).
pub const DIRECTORY_INFO: &str = "System.IO.DirectoryInfo";
/// What `GetFiles` of such a folder answers, its enumerator, and one file of it. The names are no real CLR type names, so
/// no object of the .NET host takes their members.
pub const STAGED_FILE_ARRAY: &str = "<FileInfo[] of a staged folder>";
pub const STAGED_FILE_ENUMERATOR: &str = "<enumerator of a staged folder listing>";
pub const STAGED_FILE_INFO: &str = "<FileInfo of a staged folder>";

pub const SIGS: &[(&str, &str, Ret)] = &[
    (GRAPH_QUERY, "GraphQuery", Ret::Object),
    (GRAPH_QUERY, "IsM365CollaborationEnabled", Ret::Boolean),
    (GRAPH_QUERY, "GetEnvironmentDirectoryGroup", Ret::Text),
    (GRAPH_QUERY, "GetGroupPage", Ret::Object),
    (GRAPH_QUERY, "GetGroupInfo", Ret::Object),
    (GRAPH_QUERY, "FindFirstGroupInfoForDisplayName", Ret::Object),
    (GRAPH_QUERY, "GetUser", Ret::Object),
    (GRAPH_QUERY, "GetCurrentUser", Ret::Object),
    (GRAPH_QUERY, "GetTenantDetail", Ret::Object),
    (TENANT_INFO, "InitialDomain", Ret::Text),
    (TENANT_INFO, "CountryLetterCode", Ret::Text),
    (TENANT_INFO, "PreferredLanguage", Ret::Text),
    (AL_CONFIG_SETTINGS, "Instance", Ret::Object),
    (
        AL_CONFIG_SETTINGS,
        "EnableSaasExtensionInstallConfigSetting",
        Ret::Boolean,
    ),
    (AL_CONFIG_SETTINGS, "IsSaasExcelAddinEnabled", Ret::Boolean),
    (AL_CONFIG_SETTINGS, "EnableEntraGroupsOnPrem", Ret::Boolean),
    (AL_CONFIG_SETTINGS, "ApiServicesEnabled", Ret::Boolean),
    (AL_CONFIG_SETTINGS, "ApiSubscriptionsEnabled", Ret::Boolean),
    (
        AL_CONFIG_SETTINGS,
        "ApiSubscriptionSendingNotificationTimeout",
        Ret::Integer,
    ),
    (
        AL_CONFIG_SETTINGS,
        "ApiSubscriptionMaxNumberOfNotifications",
        Ret::Integer,
    ),
    (AL_CONFIG_SETTINGS, "ApiSubscriptionDelayTime", Ret::Integer),
    (AL_CONFIG_SETTINGS, "TestAutomationEnabled", Ret::Boolean),
    (AL_CONFIG_SETTINGS, "UsePermissionsFromExtensions", Ret::Boolean),
    (AL_CONFIG_SETTINGS, "IsSaaS", Ret::Boolean),
    (TENANT_SETTINGS_HELPER, "IsSandbox", Ret::Boolean),
    (TENANT_SETTINGS_HELPER, "IsProduction", Ret::Boolean),
    (TENANT_SETTINGS_HELPER, "SetTestTenantEnvironmentType", Ret::Void),
    (TENANT_SETTINGS_HELPER, "GetEnvironmentName", Ret::Text),
    (TENANT_SETTINGS_HELPER, "GetLinkedPowerPlatformEnvironmentId", Ret::Text),
    (TENANT_SETTINGS_HELPER, "GetEnvironmentApplicationSetting", Ret::Text),
    (TENANT_SETTINGS_HELPER, "GetApplicationFamily", Ret::Text),
    (TENANT_SETTINGS_HELPER, "TryGetStringTenantSetting", Ret::Boolean),
    (USER_ACCOUNT_HELPER, "IsAzure", Ret::Boolean),
    (USER_ACCOUNT_HELPER, "IsUserDelegatedAdmin", Ret::Boolean),
    (USER_ACCOUNT_HELPER, "IsDelegatedUser", Ret::Boolean),
    (USER_ACCOUNT_HELPER, "GetEffectivePermissionForObject", Ret::Text),
    (TEST_EXECUTION, "IsInTestMode", Ret::Boolean),
    (PERMISSION_TEST_HELPER, "PermissionTestHelper", Ret::Object),
    (PERMISSION_TEST_HELPER, "Clear", Ret::Void),
    (PERMISSION_TEST_HELPER, "Dispose", Ret::Void),
    (PERMISSION_TEST_HELPER, "AddEffectivePermissionSet", Ret::Void),
    (NCL_DIALOG_EXCEPTION, "Message", Ret::Text),
    (NCL_DIALOG_EXCEPTION, "GetType", Ret::Object),
    (NCL_DIALOG_EXCEPTION, "InnerException", Ret::Object),
    (NCL_DIALOG_EXCEPTION, "ToString", Ret::Text),
    (RECORD_NOT_FOUND_EXCEPTION, "Message", Ret::Text),
    (RECORD_NOT_FOUND_EXCEPTION, "GetType", Ret::Object),
    (RECORD_NOT_FOUND_EXCEPTION, "InnerException", Ret::Object),
    (RECORD_NOT_FOUND_EXCEPTION, "ToString", Ret::Text),
    (AL_EXCEPTION, "Message", Ret::Text),
    (AL_EXCEPTION, "GetType", Ret::Object),
    (AL_EXCEPTION, "InnerException", Ret::Object),
    (AL_EXCEPTION, "ToString", Ret::Text),
    (TEST_FIELD_EXCEPTION, "Message", Ret::Text),
    (TEST_FIELD_EXCEPTION, "GetType", Ret::Object),
    (TEST_FIELD_EXCEPTION, "InnerException", Ret::Object),
    (TEST_FIELD_EXCEPTION, "ToString", Ret::Text),
    (NCL_EVALUATE_EXCEPTION, "Message", Ret::Text),
    (NCL_EVALUATE_EXCEPTION, "GetType", Ret::Object),
    (NCL_EVALUATE_EXCEPTION, "InnerException", Ret::Object),
    (NCL_EVALUATE_EXCEPTION, "ToString", Ret::Text),
    (DIVIDE_BY_ZERO_EXCEPTION, "Message", Ret::Text),
    (DIVIDE_BY_ZERO_EXCEPTION, "GetType", Ret::Object),
    (NCL_DIALOG_EXCEPTION, "GetBaseException", Ret::Object),
    (NCL_DIALOG_EXCEPTION, "Equals", Ret::Boolean),
    (RECORD_NOT_FOUND_EXCEPTION, "GetBaseException", Ret::Object),
    (RECORD_NOT_FOUND_EXCEPTION, "Equals", Ret::Boolean),
    (AL_EXCEPTION, "GetBaseException", Ret::Object),
    (AL_EXCEPTION, "Equals", Ret::Boolean),
    (TEST_FIELD_EXCEPTION, "GetBaseException", Ret::Object),
    (TEST_FIELD_EXCEPTION, "Equals", Ret::Boolean),
    (NCL_EVALUATE_EXCEPTION, "GetBaseException", Ret::Object),
    (NCL_EVALUATE_EXCEPTION, "Equals", Ret::Boolean),
    (RUNTIME_TYPE, "Equals", Ret::Boolean),
    (RUNTIME_TYPE, "FullName", Ret::Text),
    (NULL_EXCEPTION, "Message", Ret::Text),
    (NULL_EXCEPTION, "GetType", Ret::Object),
    (NULL_EXCEPTION, "ToString", Ret::Text),
    (GZIP_STREAM, "GZipStream", Ret::Object),
    (GZIP_STREAM, "CopyTo", Ret::Void),
    (GZIP_STREAM, "Dispose", Ret::Void),
    (COMPRESSION_MODE, "Compress", Ret::Object),
    (COMPRESSION_MODE, "Decompress", Ret::Object),
    (BITMAP, "Bitmap", Ret::Object),
    (BITMAP, "Save", Ret::Void),
    (BITMAP, "Dispose", Ret::Void),
    (GRAPHICS, "FromImage", Ret::Object),
    (GRAPHICS, "FillEllipse", Ret::Void),
    (GRAPHICS, "Dispose", Ret::Void),
    (COLOR_TRANSLATOR, "FromHtml", Ret::Object),
    (SOLID_BRUSH, "SolidBrush", Ret::Object),
    (IMAGE_FORMAT, "Png", Ret::Object),
    (DIRECTORY_INFO, "DirectoryInfo", Ret::Object),
    (DIRECTORY_INFO, "Exists", Ret::Boolean),
    (DIRECTORY_INFO, "GetFiles", Ret::Object),
    (STAGED_FILE_ARRAY, "GetEnumerator", Ret::Object),
    (STAGED_FILE_ENUMERATOR, "MoveNext", Ret::Boolean),
    (STAGED_FILE_ENUMERATOR, "Current", Ret::Object),
    (STAGED_FILE_INFO, "FullName", Ret::Text),
    (STAGED_FILE_INFO, "Length", Ret::Object),
];

/// The declared result of `member` on `clr_type`, if the runtime provides that member.
pub fn lookup(clr_type: &str, member: &str) -> Option<Ret> {
    SIGS.iter()
        .find(|(t, m, _)| t.eq_ignore_ascii_case(clr_type) && m.eq_ignore_ascii_case(member))
        .map(|(_, _, r)| *r)
}
