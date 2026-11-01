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
    /// A `DotNet` object (for example `Instance()`), or any result whose kind is known only at run time (a null string).
    Object,
    /// No result (a setter): the call is a statement.
    Void,
}

/// `Microsoft.Dynamics.Nav.Types.ALConfigSettings`.
pub const AL_CONFIG_SETTINGS: &str = "Microsoft.Dynamics.Nav.Types.ALConfigSettings";
/// `Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper`.
pub const TENANT_SETTINGS_HELPER: &str = "Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper";
/// Base Application's codeunit "Application System Constants": its [`PLATFORM_FILE_VERSION`] is the text the runtime's
/// `NavTenantSettingsHelper.GetPlatformVersion` parses (al-vm `dotnet/server_classes.rs` `platform_version`), and the
/// demand-driven compile reaches it for such a call (al-sema `check/demand.rs` `reach_platform_file_version`).
pub const APPLICATION_SYSTEM_CONSTANTS: u32 = 9015;
/// The procedure of [`APPLICATION_SYSTEM_CONSTANTS`] that `GetPlatformVersion` calls.
pub const PLATFORM_FILE_VERSION: &str = "PlatformFileVersion";
/// `Microsoft.Dynamics.Nav.Service.CopilotApi.AL.ALCopilotFunctions`, behind System Application's "AOAI Authorization".
pub const COPILOT_FUNCTIONS: &str = "Microsoft.Dynamics.Nav.Service.CopilotApi.AL.ALCopilotFunctions";
/// `Microsoft.Dynamics.Nav.Runtime.NavTestExecution`.
pub const TEST_EXECUTION: &str = "Microsoft.Dynamics.Nav.Runtime.NavTestExecution";
/// `Microsoft.Dynamics.Nav.AzureKeyVaultClient.AzureKeyVaultAppSecretProvider`, behind System Application "App Key Vault Secret Pr. Impl.".
pub const AKV_PROVIDER: &str = "Microsoft.Dynamics.Nav.AzureKeyVaultClient.AzureKeyVaultAppSecretProvider";
/// `Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper`, which codeunit 131006 "Permissions Mock" constructs.
pub const PERMISSION_TEST_HELPER: &str = "Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper";
/// `Microsoft.Dynamics.Nav.NavUserAccount.NavUserAccountHelper`.
/// Azure AD Graph's query object (System Application "Azure AD Graph Impl.").
pub const GRAPH_QUERY: &str = "Microsoft.Dynamics.Nav.AzureADGraphClient.GraphQuery";
/// `Microsoft.Dynamics.Nav.LicensingService.Model.TenantInfo` (the answer of `GraphQuery.GetTenantDetail`).
pub const TENANT_INFO: &str = "Microsoft.Dynamics.Nav.LicensingService.Model.TenantInfo";
pub const USER_ACCOUNT_HELPER: &str = "Microsoft.Dynamics.Nav.NavUserAccount.NavUserAccountHelper";
/// `System.Environment`: only `GetEnvironmentVariable` is served, and it answers "not set" for every variable (owner
/// decision 2026-10-30, issue 6222). Its result is a null string, so it is typed as a value known at run time (`Object`).
pub const ENVIRONMENT: &str = "System.Environment";

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
/// What a `var` parameter holds while it stands for a `[RunOnClient]` variable: the null of a client-side variable. A test
/// session has no client, so the variable never holds an object; the marker only tells the calls on the parameter that
/// they run on the client (corpus master 92fdde19, onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al:282-302,
/// Y16_RunOnClient_CreateThroughVarParameter). `IsNull` is true.
pub const RUN_ON_CLIENT_NULL: &str = "<null of a [RunOnClient] variable>";
/// The error of a `[RunOnClient]` call in a test session (onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al:132-133).
pub const DOTNET_FROM_AL_NOT_SUPPORTED_TEXT: &str =
    "The operation cannot complete because it uses a feature that is not supported (.NET Framework Interoperability.)";
pub const DOTNET_FROM_AL_NOT_SUPPORTED_CODE: &str = "DotNetFromAlNotSupported:DotNetFromAlNotSupported";

/// The client feature types whose static `IsAvailable()` answers No in a test session, on a `[RunOnClient]` variable
/// (onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al:99-120, the seven types of the probe).
pub fn is_client_capability(clr_type: &str) -> bool {
    [
        "Microsoft.Dynamics.Nav.Client.Capabilities.CameraProvider",
        "Microsoft.Dynamics.Nav.Client.Capabilities.LocationProvider",
        "Microsoft.Dynamics.Nav.Client.Capabilities.BarcodeScannerProvider",
        "Microsoft.Dynamics.Nav.Client.Capabilities.CameraBarcodeScannerProvider",
        "Microsoft.Dynamics.Nav.Client.Capabilities.DeviceContactProvider",
        "Microsoft.Dynamics.Nav.Client.Capabilities.UserTours",
        "Microsoft.Dynamics.Nav.Client.PageNotifier",
    ]
    .contains(&clr_type)
}

/// The members of a `[RunOnClient]` variable whose call raises DotNetFromAlNotSupported in a test session (besides the
/// constructor): `Create` (onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al:186-209, :282-302), Environment.MachineName (:138-150)
/// and System.IO.Directory's Exists and GetFiles (:363-391, Y16_RunOnClient_FileManagementDirectory). Others are not pinned.
pub fn run_on_client_fails(member: &str) -> bool {
    ["Create", "MachineName", "Exists", "GetFiles"]
        .iter()
        .any(|m| m.eq_ignore_ascii_case(member))
}

/// Microsoft's client camera (System Application Camera Impl.): on the service tier a registry type (`server_classes.rs`).
pub const CAMERA_PROVIDER: &str = "Microsoft.Dynamics.Nav.Client.Capabilities.CameraProvider";
/// What `GetType()` of a last-error exception answers. The name must not be a real CLR type name: the host
/// reports `System.RuntimeType` for the `System.Type` objects it holds (GetDotNetType), and a registry type of
/// that name would take their members.
pub const RUNTIME_TYPE: &str = "<Type of a last-error exception>";

/// What an AL `OutStream` or `InStream` is when a `DotNet` System.IO.Stream variable holds it: its run-time type name
/// (tests/al-language-onprem-dotnet/lt-sg-dotnet/SgDotNetStreams.al:118, :387), served by al-vm `dotnet/blob_stream.rs`.
pub const NAV_BLOB_STREAM: &str = "Microsoft.Dynamics.Nav.Runtime.NavBLOB+InternalStream";
/// What `GetType()` of that stream answers (only `FullName` is pinned). Not a real CLR type name, as for [`RUNTIME_TYPE`].
pub const NAV_BLOB_STREAM_TYPE: &str = "<Type of NavBLOB+InternalStream>";

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

/// Microsoft's reader of a query's metadata (al-vm dotnet/query_metadata.rs), the list its `Fields()` answers (no real CLR
/// type name, so no object of the .NET host takes its members), and one entry of it.
/// AL declares it without type arguments (Base Application's ODataUtility, al-vm `dotnet/generic_dictionary.rs`).
pub const GENERIC_DICTIONARY: &str = "System.Collections.Generic.Dictionary`2";
pub const GENERIC_DICTIONARY_ENUMERATOR: &str = "<enumerator of a runtime-made Dictionary`2>";
/// What `foreach` over such a dictionary yields.
pub const GENERIC_KEY_VALUE_PAIR: &str = "System.Collections.Generic.KeyValuePair`2";
pub const QUERY_METADATA_READER: &str = "Microsoft.Dynamics.Nav.Client.BusinessChart.QueryMetadataReader";
pub const QUERY_FIELDS_LIST: &str = "<fields of a query metadata reader>";
pub const QUERY_FIELDS: &str = "Microsoft.Dynamics.Nav.Client.BusinessChart.QueryFields";

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
    (TENANT_SETTINGS_HELPER, "GetPlatformVersion", Ret::Object),
    (TENANT_SETTINGS_HELPER, "TryGetStringTenantSetting", Ret::Boolean),
    (USER_ACCOUNT_HELPER, "IsAzure", Ret::Boolean),
    (USER_ACCOUNT_HELPER, "IsUserDelegatedAdmin", Ret::Boolean),
    (USER_ACCOUNT_HELPER, "IsDelegatedUser", Ret::Boolean),
    (USER_ACCOUNT_HELPER, "IsUserDelegatedHelpdesk", Ret::Boolean),
    (USER_ACCOUNT_HELPER, "IsWindowsAuthentication", Ret::Boolean),
    (USER_ACCOUNT_HELPER, "IsUserNamePasswordAuthentication", Ret::Boolean),
    (
        USER_ACCOUNT_HELPER,
        "IsAccessControlServiceAuthentication",
        Ret::Boolean,
    ),
    (USER_ACCOUNT_HELPER, "GetEffectivePermissionForObject", Ret::Text),
    (USER_ACCOUNT_HELPER, "IsPermissionSetAssigned", Ret::Boolean),
    (AKV_PROVIDER, "AzureKeyVaultAppSecretProvider", Ret::Object),
    (
        AKV_PROVIDER,
        "InitializeFromCallingAppExceptSystemApplication",
        Ret::Void,
    ),
    (AKV_PROVIDER, "TryGetAzureKeyVaultSecret", Ret::Boolean),
    (TEST_EXECUTION, "IsInTestMode", Ret::Boolean),
    (COPILOT_FUNCTIONS, "IsPlatformAuthorizationConfigured", Ret::Boolean),
    (ENVIRONMENT, "GetEnvironmentVariable", Ret::Object),
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
    (CAMERA_PROVIDER, "IsAvailable", Ret::Boolean),
    (CAMERA_PROVIDER, "Create", Ret::Object),
    (NULL_EXCEPTION, "Message", Ret::Text),
    (NULL_EXCEPTION, "GetType", Ret::Object),
    (NULL_EXCEPTION, "ToString", Ret::Text),
    (NAV_BLOB_STREAM, "GetType", Ret::Object),
    (NAV_BLOB_STREAM, "CanRead", Ret::Boolean),
    (NAV_BLOB_STREAM, "CanWrite", Ret::Boolean),
    (NAV_BLOB_STREAM, "CanSeek", Ret::Boolean),
    (NAV_BLOB_STREAM, "Length", Ret::Object),
    (NAV_BLOB_STREAM, "Position", Ret::Object),
    (NAV_BLOB_STREAM, "ReadByte", Ret::Integer),
    (NAV_BLOB_STREAM, "WriteByte", Ret::Void),
    (NAV_BLOB_STREAM, "Write", Ret::Void),
    (NAV_BLOB_STREAM, "Flush", Ret::Void),
    (NAV_BLOB_STREAM, "Close", Ret::Void),
    (NAV_BLOB_STREAM, "Dispose", Ret::Void),
    (NAV_BLOB_STREAM_TYPE, "FullName", Ret::Text),
    (GZIP_STREAM, "GZipStream", Ret::Object),
    (GZIP_STREAM, "CopyTo", Ret::Void),
    (GZIP_STREAM, "Dispose", Ret::Void),
    (GZIP_STREAM, "Close", Ret::Void),
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
    (GENERIC_DICTIONARY, "Dictionary", Ret::Object),
    (GENERIC_DICTIONARY, "Add", Ret::Void),
    (GENERIC_DICTIONARY, "ContainsKey", Ret::Boolean),
    (GENERIC_DICTIONARY, "Count", Ret::Integer),
    (GENERIC_DICTIONARY, "Item", Ret::Object),
    (GENERIC_KEY_VALUE_PAIR, "Key", Ret::Object),
    (GENERIC_KEY_VALUE_PAIR, "Value", Ret::Object),
    (QUERY_METADATA_READER, "FromStream", Ret::Object),
    (QUERY_METADATA_READER, "Fields", Ret::Object),
    (QUERY_FIELDS_LIST, "Count", Ret::Integer),
    (QUERY_FIELDS_LIST, "Item", Ret::Object),
    (QUERY_FIELDS, "FieldNo", Ret::Integer),
    (QUERY_FIELDS, "TableNo", Ret::Integer),
    (QUERY_FIELDS, "FieldName", Ret::Text),
];

/// The declared result of `member` on `clr_type`, if the runtime provides that member.
pub fn lookup(clr_type: &str, member: &str) -> Option<Ret> {
    SIGS.iter()
        .find(|(t, m, _)| t.eq_ignore_ascii_case(clr_type) && m.eq_ignore_ascii_case(member))
        .map(|(_, _, r)| *r)
}
