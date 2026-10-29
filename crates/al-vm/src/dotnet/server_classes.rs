//! The BC server classes Microsoft's AL reaches through `DotNet` (ADR-014 decision 2, owner
//! decision 2026-10-14). They cannot work outside a service tier: they read the session, the
//! tenant or the service configuration. The runtime provides them itself, so they are never
//! forwarded to the sidecar.
//!
//! A value measured on real BC is pinned by a probe test of the corpus (`Pin::Probe`, a path under
//! `tests/` with its line). The rest mirrors AL Runner v2 until a probe covers it (`Pin::AlRunner`, with the
//! AL Runner file and line). AL Runner runs BC's real classes on a faked session, so a value is
//! what the real class answers on that session. Where the probe and AL Runner differ, the probe
//! wins: the oracle tenant is a sandbox on Azure-like topology, AL Runner's is Production on
//! premises.
//!
//! The result type of each member is declared in `al_builtins::dotnet_sigs`; a test in
//! `tests/dotnet.rs` checks that both lists name the same members.

use super::registry::{MemberFn, MemberKind, MemberRefFn, Pin, RegistryMember, RegistryType};
use crate::Session;
use al_builtins::dotnet_sigs::{
    AKV_PROVIDER, AL_CONFIG_SETTINGS, COPILOT_FUNCTIONS, ENVIRONMENT, GRAPH_QUERY, PERMISSION_TEST_HELPER, TENANT_INFO,
    TENANT_SETTINGS_HELPER, TEST_EXECUTION, USER_ACCOUNT_HELPER,
};
use al_builtins::native::Signal;
use al_builtins::value::{DotNetKind, DotNetObject, Value};
use std::cell::RefCell;
use std::rc::Rc;

type Args<'a> = &'a [Value];

fn no_args(args: Args, what: &str) -> Result<(), Signal> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(Signal::Unsupported(format!(
            "DotNet {what} takes no arguments here; the call has {}",
            args.len()
        )))
    }
}

/// The `ALConfigSettings` flags are measured on real BC 27.5. They equal the `ConfigSetting`
/// attribute defaults of `ServerUserSettings` (Nav.Types 27.5), which AL Runner v2 answers.
const CFG_PIN: &str = "probe tests/al-language-onprem/session/TestServerSettingContracts.al:31-48; \
    AL Runner v2 agrees: it patches nothing here (no ALConfigSettings in AlRunner/), \
    so the real ServerUserSettings answers its attribute default, and AlRunner/Patches/\
    RecordPatches.AggregatePermissionSetVirtualTable.cs:9 states UsePermissionSetsFromExtensions is true";

macro_rules! config_flag {
    ($name:ident, $what:literal, $value:expr) => {
        fn $name(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
            no_args(args, $what)?;
            Ok($value)
        }
    };
}

fn al_config_instance(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "ALConfigSettings.Instance")?;
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: AL_CONFIG_SETTINGS.into(),
        state: RefCell::new(Vec::new()),
    }))))
}

// ServerUserSettings attribute defaults (Nav.Types 27.5): EnableSaasExtensionInstall false,
// ExcelAddInAzureActiveDirectoryClientId "" (no Guid, so false), EnableEntraGroupsOnPrem false,
// ApiServicesEnabled true, ApiSubscriptionsEnabled true, ApiSubscriptionSendingNotificationTimeout
// 30000, ApiSubscriptionMaxNumberOfNotifications 1000, ApiSubscriptionDelayTime 30000,
// TestAutomationEnabled true, UsePermissionSetsFromExtensions true (also stated in AL Runner's
// RecordPatches.AggregatePermissionSetVirtualTable.cs:9), EnableMembershipEntitlement false.
config_flag!(
    save_ext_install,
    "ALConfigSettings.EnableSaasExtensionInstallConfigSetting",
    Value::Boolean(false)
);
config_flag!(
    excel_addin,
    "ALConfigSettings.IsSaasExcelAddinEnabled",
    Value::Boolean(false)
);
config_flag!(
    entra_groups,
    "ALConfigSettings.EnableEntraGroupsOnPrem",
    Value::Boolean(false)
);
config_flag!(
    api_services,
    "ALConfigSettings.ApiServicesEnabled",
    Value::Boolean(true)
);
config_flag!(
    api_subscriptions,
    "ALConfigSettings.ApiSubscriptionsEnabled",
    Value::Boolean(true)
);
config_flag!(
    api_sending_timeout,
    "ALConfigSettings.ApiSubscriptionSendingNotificationTimeout",
    Value::Integer(30000)
);
config_flag!(
    api_max_notifications,
    "ALConfigSettings.ApiSubscriptionMaxNumberOfNotifications",
    Value::Integer(1000)
);
config_flag!(
    api_delay,
    "ALConfigSettings.ApiSubscriptionDelayTime",
    Value::Integer(30000)
);
config_flag!(
    test_automation,
    "ALConfigSettings.TestAutomationEnabled",
    Value::Boolean(true)
);
config_flag!(
    permission_sets_from_extensions,
    "ALConfigSettings.UsePermissionsFromExtensions",
    Value::Boolean(true)
);
config_flag!(membership_entitlement, "ALConfigSettings.IsSaaS", Value::Boolean(false));

/// Whether the tenant is a sandbox. The SaaS environment is the oracle tier (probe: `IsSandbox`
/// true, `IsProduction` false, `IsSaaS` true, no testability flag set). AL Runner's skeleton tenant
/// is Production; the probe wins. The on-prem environment (`--environment onprem`, owner decision
/// 2026-10-15) is not a sandbox: Microsoft's codeunit 134614 "Test App Permissions" deletes all
/// users before it sets `SetTestabilitySoftwareAsAService(true)`, which passes only when `IsSaaS`
/// is false until then. UNPROBED ASSUMPTION (docs/probes-needed.md "Environment Information on an
/// on-prem server"); the OnPrem suite probe, when it lands, wins.
fn tenant_is_sandbox(s: &Session) -> bool {
    s.host.identity.environment() == al_host::Environment::Saas
}

fn is_sandbox(s: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "NavTenantSettingsHelper.IsSandbox")?;
    Ok(Value::Boolean(tenant_is_sandbox(s)))
}

/// BC's body is `!IsSandbox()`.
fn is_production(s: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "NavTenantSettingsHelper.IsProduction")?;
    Ok(Value::Boolean(!tenant_is_sandbox(s)))
}

fn text(what: &str, args: Args, want: usize, v: &str) -> Result<Value, Signal> {
    if args.len() != want {
        return Err(Signal::Unsupported(format!(
            "DotNet {what} takes {want} argument(s) here; the call has {}",
            args.len()
        )));
    }
    Ok(Value::Text(v.into()))
}

/// `SetTestTenantEnvironmentType(Boolean)`: what the test library's `SetTestabilitySandbox` /
/// `SetTestabilitySoftwareAsAService` reach changes nothing a test can see on the probe tier
/// (corpus tests/al-language-onprem/session/PVEnvironmentTestability.al, staging 8217373: after
/// SetTestabilitySandbox(false) the session still reads IsSandbox Yes, IsProduction No and the name
/// "Sandbox"). So the setter is accepted and does nothing.
fn set_test_tenant_environment_type(
    _: &mut Session,
    _: Option<&Rc<DotNetObject>>,
    args: Args,
) -> Result<Value, Signal> {
    match args {
        [Value::Boolean(_)] => Ok(Value::Null),
        _ => Err(Signal::Unsupported(
            "DotNet NavTenantSettingsHelper.SetTestTenantEnvironmentType takes one Boolean here".into(),
        )),
    }
}

/// The wrapper answers 'Sandbox' (probe). Whether the helper returns that or an empty name that
/// the wrapper turns into 'Sandbox' is not observable through the wrapper; the empty name is the
/// default of an unset `NavTenantSettings.EnvironmentName`.
fn environment_name(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    text("NavTenantSettingsHelper.GetEnvironmentName", args, 0, "")
}

/// `TryGetStringTenantSetting(name, out value)`: the three settings System Application reads, with
/// the values the oracle tenant answers through the wrappers `Tenant Information.GetTenantId` and
/// `GetTenantDisplayName` and `Azure AD Tenant.GetAadTenantId`
/// (session/TestEnvironmentUserTenantContracts.al:TenantAndAzureAd_Answers, :209-212). The
/// wrappers return the out value as it is; `GetAadTenantId` returns it when it is not a Guid, so
/// the setting itself is `common`. Any other name is refused: nothing pins its value or whether
/// the call succeeds.
fn try_get_string_tenant_setting(
    s: &mut Session,
    _: Option<&Rc<DotNetObject>>,
    args: &mut [Value],
) -> Result<Value, Signal> {
    let [Value::Text(name) | Value::Code(name), out] = args else {
        return Err(Signal::Unsupported(
            "DotNet NavTenantSettingsHelper.TryGetStringTenantSetting takes (Text, var Text)".into(),
        ));
    };
    let value = match name.to_ascii_uppercase().as_str() {
        "TENANTID" => s.host.identity.tenant_id(),
        "DISPLAYNAME" => String::new(),
        "AADTENANTID" => "common".to_string(),
        _ => {
            return Err(Signal::Unsupported(format!(
                "DotNet NavTenantSettingsHelper.TryGetStringTenantSetting('{name}'): the setting is not pinned \
                 (docs/probes-needed.md)"
            )));
        }
    };
    *out = Value::Text(value.into());
    Ok(Value::Boolean(true))
}

fn application_family(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    text("NavTenantSettingsHelper.GetApplicationFamily", args, 0, "W1")
}

fn power_platform_environment_id(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    text(
        "NavTenantSettingsHelper.GetLinkedPowerPlatformEnvironmentId",
        args,
        0,
        "",
    )
}

/// Every name maps to `GetEnvironmentSetting`, which reads the empty `EnvironmentSettings`.
fn environment_application_setting(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    text("NavTenantSettingsHelper.GetEnvironmentApplicationSetting", args, 1, "")
}

/// `!NavEnvironment.Topology.IsServiceRunningInLocalEnvironment`, which is
/// `Environment Information.IsSaaSInfrastructure`. The oracle answers true on BC 27.5 and 28.5 and
/// false on 29.0 (`Gate::NotSaaSInfrastructure`; corpus
/// `session/TestEnvironmentUserTenantContracts.al:164-174`). AL Runner's `StandardServiceTopology`
/// answers false throughout; the probe wins where it says true.
fn is_azure(s: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "NavUserAccountHelper.IsAzure")?;
    Ok(Value::Boolean(!al_version::gate_on(
        s.engine.target.as_ref(),
        al_version::Gate::NotSaaSInfrastructure,
    )))
}

/// `NavUserAccountHelper.IsUserDelegatedAdmin()`: false in a test session on every supported major (corpus
/// `al-language-onprem-dotnet/lt-al-dotnet/AlUserAccountHelper.al:62`; through `Azure AD Graph User`:
/// `lt-ae-dotnet/AeUserAccountHelper.al:23`).
fn is_user_delegated_admin(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "NavUserAccountHelper.IsUserDelegatedAdmin")?;
    Ok(Value::Boolean(false))
}

/// `NavUserAccountHelper.IsDelegatedUser()`: false in a test session on every supported major (corpus
/// `al-language-onprem-dotnet/lt-al-dotnet/AlUserAccountHelper.al:61`).
fn is_delegated_user(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "NavUserAccountHelper.IsDelegatedUser")?;
    Ok(Value::Boolean(false))
}

/// `NavUserAccountHelper.IsUserDelegatedHelpdesk()`: false in a test session on every supported major (corpus
/// `al-language-onprem-dotnet/lt-al-dotnet/AlUserAccountHelper.al:63`; through `Azure AD Graph User`:
/// `lt-ae-dotnet/AeUserAccountHelper.al:24`). System Application's `Azure AD User Mgmt. Impl.` calls it on company open.
fn is_user_delegated_helpdesk(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "NavUserAccountHelper.IsUserDelegatedHelpdesk")?;
    Ok(Value::Boolean(false))
}

/// `NavUserAccountHelper.GetEffectivePermissionForObject(user, company, objectType, objectId)`:
/// five comma-separated Permission ordinals (Read, Insert, Modify, Delete, Execute) of the
/// session user. A test session is SUPER; on table data it holds the four data rights and no
/// Execute (corpus session/TestSessionCompanyInfoExperienceClose.al:62-90, measured on real BC).
/// Other object kinds and other users are not pinned and are refused.
fn effective_permission(s: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    let [
        Value::Guid(user),
        Value::Text(_) | Value::Code(_),
        kind,
        Value::Integer(_),
    ] = args
    else {
        return Err(Signal::Unsupported(
            "DotNet NavUserAccountHelper.GetEffectivePermissionForObject takes (Guid, Text, Option, Integer)".into(),
        ));
    };
    if *user != s.host.identity.user_security_id() {
        return Err(Signal::Unsupported(
            "DotNet NavUserAccountHelper.GetEffectivePermissionForObject for another user: no per-user \
             permission state is held"
                .into(),
        ));
    }
    match kind {
        Value::Option(0, _) | Value::Integer(0) => Ok(Value::Text("1,1,1,1,0".into())),
        _ => Err(Signal::Unsupported(
            "DotNet NavUserAccountHelper.GetEffectivePermissionForObject on an object kind other than table \
             data: the rights of a SUPER session are not pinned (docs/probes-needed.md)"
                .into(),
        )),
    }
}

/// `NavUserAccountHelper.IsPermissionSetAssigned(user, company, roleId, appId, scope)`: false when no Access Control row of that
/// user and role is for that company or all companies with that app id (platform/lt-outside-platform/RvPermissionSetAssigned.al:
/// 74-200, through "User Permissions".HasUserPermissionSetAssigned, AL Runner issue 5796: a set not held, a row for another company,
/// a row with another app id). "User Permissions" finds a matching row itself before it asks, so what the member answers for one
/// is not pinned and is refused; so are a row with another scope, a company that differs only in case, and an assignment
/// through a security group (the corpus CI has none; the runtime holds none either).
fn is_permission_set_assigned(s: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    let text = |v: &Value| al_builtins::native::text_of(v).map(|t| t.to_string());
    let ordinal = |v: &Value| match v {
        Value::Option(n, _) | Value::Integer(n) => Some(*n),
        _ => None,
    };
    let [Value::Guid(user), company, role, Value::Guid(app), scope] = args else {
        return Err(Signal::Unsupported(
            "DotNet NavUserAccountHelper.IsPermissionSetAssigned takes (Guid, Text, Code, Guid, Option)".into(),
        ));
    };
    let (Some(company), Some(role), Some(scope)) = (text(company), text(role), ordinal(scope)) else {
        return Err(Signal::Unsupported(
            "DotNet NavUserAccountHelper.IsPermissionSetAssigned takes (Guid, Text, Code, Guid, Option)".into(),
        ));
    };
    let Some(ac) = s.table_def(2_000_000_053) else {
        return Err(Signal::Unsupported(
            "DotNet NavUserAccountHelper.IsPermissionSetAssigned: no Access Control table".into(),
        ));
    };
    let col = |name: &str| ac.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name));
    let (Some(c_user), Some(c_role), Some(c_company), Some(c_scope), Some(c_app)) = (
        col("User Security ID"),
        col("Role ID"),
        col("Company Name"),
        col("Scope"),
        col("App ID"),
    ) else {
        return Err(Signal::Unsupported(
            "DotNet NavUserAccountHelper.IsPermissionSetAssigned: an Access Control column is missing".into(),
        ));
    };
    let cx = s.cx.clone();
    for r in s.all_rows(&cx, &ac)? {
        let same_guid = |i: usize, g: &[u8; 16]| matches!(&r[i], Value::Guid(x) if x == g);
        if !same_guid(c_user, user) || !text(&r[c_role]).is_some_and(|x| x.eq_ignore_ascii_case(&role)) {
            continue;
        }
        if ordinal(&r[c_scope]) != Some(scope) {
            return Err(Signal::Unsupported(
                "DotNet NavUserAccountHelper.IsPermissionSetAssigned with a row of another scope (not pinned)".into(),
            ));
        }
        let row_company = text(&r[c_company]).unwrap_or_default();
        if same_guid(c_app, app) && (row_company.is_empty() || row_company.eq_ignore_ascii_case(&company)) {
            return Err(Signal::Unsupported(
                "DotNet NavUserAccountHelper.IsPermissionSetAssigned for a set an Access Control row assigns (what it answers is not pinned)"
                    .into(),
            ));
        }
    }
    Ok(Value::Boolean(false))
}

/// `NavTestExecution.IsInTestMode()`: true while a test codeunit runs. The probe's
/// `Environment Information.CanStartSession` is false, which is `TaskScheduler.CanCreateTask`
/// (false, also probed) behind this being true.
fn is_in_test_mode(s: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "NavTestExecution.IsInTestMode")?;
    Ok(Value::Boolean(s.in_test_codeunit))
}

/// `ALCopilotFunctions.IsPlatformAuthorizationConfigured(callerPublisher, systemApplicationPublisher)`, which System Application's
/// "AOAI Authorization".IsConfigured asks for First Party authorization: false, with no error, for a caller published by
/// 'AL Language' and the System Application published by 'Microsoft' (corpus platform/lt-ri-aoai/RiAoaiAuthorization.al:44-94,
/// BC 27, 28 and 29; al-runner issue 5693). A Microsoft caller is not pinned: whether the platform authorizes its own
/// publisher's Copilot features on the test tier is refused.
fn copilot_platform_authorization(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    let publisher = |v: &Value| al_builtins::native::text_of(v).map(|t| t.to_string());
    match args {
        [caller, current]
            if publisher(current).as_deref() == Some("Microsoft")
                && publisher(caller).is_some_and(|c| !c.trim().eq_ignore_ascii_case("microsoft")) =>
        {
            Ok(Value::Boolean(false))
        }
        _ => Err(Signal::Unsupported(
            "DotNet ALCopilotFunctions.IsPlatformAuthorizationConfigured for a Microsoft caller or other arguments (not pinned)"
                .into(),
        )),
    }
}

const AKV_PIN: &str = "onprem-dotnet/lt-ty-dotnet/TyAkvProvider.al:84-104 (construct, initialize), :108-120 (TryGetAzureKeyVaultSecret \
    raises \"Not initialized.\" before and after a failed initialize); registered assumption akv-provider-no-key-vault";

/// `new AzureKeyVaultAppSecretProvider()`: an object (TyAkvProvider.al:84-104, 'construct' gives isnull=No).
fn akv_new(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "AzureKeyVaultAppSecretProvider constructor")?;
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: AKV_PROVIDER.into(),
        state: RefCell::new(Vec::new()),
    }))))
}

/// `InitializeFromCallingAppExceptSystemApplication()`: no app has a key vault here, so it raises
/// `No key vaults specified for extension '<name> by <publisher> <version>'.` (TyAkvProvider.al:84-104; code
/// AzureKeyVaultAppSecretProvider). The app is the first one on the call stack that is not the System Application.
fn akv_initialize(s: &mut Session, this: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(
        args,
        "AzureKeyVaultAppSecretProvider.InitializeFromCallingAppExceptSystemApplication",
    )?;
    if this.is_none() {
        return Err(Signal::Unsupported(
            "DotNet AzureKeyVaultAppSecretProvider.InitializeFromCallingAppExceptSystemApplication on a variable never constructed (not pinned; only TryGetAzureKeyVaultSecret is)"
                .into(),
        ));
    }
    // Pinned by tests/al-language-onprem-dotnet/lt-ty-dotnet/TyAkvProvider.al TY_Akv_Construct_Initialize, TY_Akv_TryGetSecret_NoInitialize (was assumption `akv-provider-no-key-vault`).
    let program = &s.engine.program;
    let app = s
        .frames
        .iter()
        .rev()
        .filter_map(|o| program.apps.get(program.objects[*o].app as usize))
        .find(|a| !a.name.eq_ignore_ascii_case("System Application"));
    let Some(app) = app else {
        return Err(Signal::Unsupported(
            "DotNet AzureKeyVaultAppSecretProvider.InitializeFromCallingAppExceptSystemApplication: no app other than the System Application on the call stack"
                .into(),
        ));
    };
    al_builtins::native::raise(
        format!(
            "No key vaults specified for extension '{} by {} {}'.",
            app.name, app.publisher, app.version
        ),
        "AzureKeyVaultAppSecretProvider",
    )
}

/// `TryGetAzureKeyVaultSecret(name, var value)`: the provider never initializes, so it raises `Not initialized.`
/// and leaves the out value as it was (TyAkvProvider.al:108-120).
fn akv_try_get(s: &mut Session, this: Option<&Rc<DotNetObject>>, _: Args) -> Result<Value, Signal> {
    // Pinned by tests/al-language-onprem-dotnet/lt-ty-dotnet/TyAkvProvider.al TY_Akv_Construct_Initialize, TY_Akv_TryGetSecret_NoInitialize (was assumption `akv-provider-no-key-vault`).
    if this.is_none() {
        // A variable never constructed (TyAkvProvider.al:140-152, TY_Akv_TryGetSecret_NotConstructed): the error names the
        // routine of the codeunit that made the call.
        let Some((&object, &frame)) = s.frames.last().zip(s.call_stack.last()) else {
            return Err(Signal::Unsupported(
                "DotNet AzureKeyVaultAppSecretProvider.TryGetAzureKeyVaultSecret on a variable never constructed, outside a routine"
                    .into(),
            ));
        };
        let o = &s.engine.program.objects[object];
        if o.kind != al_ir::ObjKind::Codeunit || o.uncompiled {
            return Err(Signal::Unsupported(
                "DotNet AzureKeyVaultAppSecretProvider.TryGetAzureKeyVaultSecret on a variable never constructed, outside a codeunit (the object kind's spelling in the error is not pinned)"
                    .into(),
            ));
        }
        let routine = o.routines.get(frame.routine).map_or("", |r| r.name.as_str());
        return al_builtins::native::raise(
            format!(
                "A DotNet variable has not been instantiated. Attempting to call {AKV_PROVIDER}.TryGetAzureKeyVaultSecret in CodeUnit {}: {routine}",
                o.name
            ),
            "DotNetVariableNotCreated",
        );
    }
    al_builtins::native::raise("Not initialized.", "AzureKeyVaultAppSecretProvider")
}

const ENV_PROBE: &str = "probe tests/al-language/session/TestEnvironmentUserTenantContracts.al";

fn member(name: &str, pin: Pin, f: MemberFn) -> RegistryMember {
    RegistryMember {
        name: name.into(),
        kind: MemberKind::Call,
        pin,
        call: f,
        call_ref: None,
    }
}

/// A member with `var`/`out` arguments (see [`MemberRefFn`]).
fn member_ref(name: &str, pin: Pin, f: MemberRefFn) -> RegistryMember {
    fn unused(_: &mut Session, _: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
        Err(Signal::Unsupported(
            "a member with var arguments called without them".into(),
        ))
    }
    RegistryMember {
        call_ref: Some(f),
        ..member(name, pin, unused)
    }
}

/// Azure AD Graph on a tier without Microsoft Entra ID, probed on BC 27.5 (corpus branch
/// al-runtime-rs-staging, tests/al-language-onprem/session/PXAzureAdGraph.al:1-12, 86-109).
const GRAPH_PIN: &str =
    "probe tests/al-language-onprem/session/PXAzureAdGraph.al:1-12,86-109 (corpus branch al-runtime-rs-staging)";

fn graph_query_new(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "GraphQuery constructor")?;
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: GRAPH_QUERY.into(),
        state: RefCell::new(Vec::new()),
    }))))
}

fn graph_collaboration(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "GraphQuery.IsM365CollaborationEnabled")?;
    Ok(Value::Boolean(false))
}

fn graph_directory_group(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "GraphQuery.GetEnvironmentDirectoryGroup")?;
    Ok(Value::Text(String::new().into()))
}

/// A call that needs the directory: the tier has none, the client object inside is null.
fn graph_null_reference(member: &str) -> Result<Value, Signal> {
    al_builtins::native::raise(
        format!(
            "A call to {GRAPH_QUERY}.{member} failed with this message: Object reference not set to an instance of an object."
        ),
        "DotNetInvoke:NullReference",
    )
}

fn graph_group_page(_: &mut Session, _: Option<&Rc<DotNetObject>>, _: Args) -> Result<Value, Signal> {
    graph_null_reference("GetGroupPage")
}

fn graph_group_info(_: &mut Session, _: Option<&Rc<DotNetObject>>, _: Args) -> Result<Value, Signal> {
    graph_null_reference("GetGroupInfo")
}

fn graph_first_group(_: &mut Session, _: Option<&Rc<DotNetObject>>, _: Args) -> Result<Value, Signal> {
    graph_null_reference("FindFirstGroupInfoForDisplayName")
}

/// `GetCurrentUser()` on a tier without Entra ID returns without an error and leaves the UserInfo null (probe agent,
/// OnPrem codeunit 61296 of the newest major, tests/al-language-onprem/session/PZAzureAdUser.al): Microsoft's
/// `InitializeGraphUser` then asks for the authentication object id of the session user.
fn graph_current_user(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "GraphQuery.GetCurrentUser")?;
    Ok(Value::Null)
}

/// `GetTenantDetail()` answers a TenantInfo whose three strings are empty (see `TENANT_INFO_PIN`).
fn graph_tenant_detail(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "GraphQuery.GetTenantDetail")?;
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: TENANT_INFO.into(),
        state: RefCell::new(Vec::new()),
    }))))
}

/// Microsoft's Azure AD Tenant Impl. raises "Failed to retrieve ..." for a null TenantInfo, and the corpus test
/// expects '' from `GetAadTenantDomainName`, `GetCountryLetterCode` and `GetPreferredLanguage`.
const TENANT_INFO_PIN: &str = "session/TestEnvironmentUserTenantContracts.al:213-232 (GetAadTenantDomainName, GetCountryLetterCode and GetPreferredLanguage answer '' without an error)";

fn tenant_info_empty_text(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "TenantInfo member")?;
    Ok(Value::Text(String::new().into()))
}

fn graph_user(_: &mut Session, _: Option<&Rc<DotNetObject>>, _: Args) -> Result<Value, Signal> {
    graph_null_reference("GetUser")
}

/// Evidence for the members of `PermissionTestHelper` (the platform object behind codeunit 131006 "Permissions Mock").
const MOCK_LIFECYCLE_PIN: &str = "permissions/TestPermissionsMockLifecycle.al:29-82 (Start, ClearAssignments and Stop are \
    accepted); permissions/TestPermissionsMockEnforcement.al:74-97 (with no set assigned, the user's own permissions \
    apply: the insert goes through)";

/// `PermissionTestHelper()`: starts the mock with no set assigned. The user's own permissions apply then
/// (`MOCK_LIFECYCLE_PIN`), and every access is granted here (ADR-027), so the object holds nothing.
fn permission_helper_new(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "PermissionTestHelper constructor")?;
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: PERMISSION_TEST_HELPER.into(),
        state: RefCell::new(Vec::new()),
    }))))
}

/// `Clear()` and `Dispose()`: back to no assigned set, the user's own permissions (`MOCK_LIFECYCLE_PIN`).
fn permission_helper_clear(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "PermissionTestHelper.Clear")?;
    Ok(Value::Null)
}

fn permission_helper_dispose(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "PermissionTestHelper.Dispose")?;
    Ok(Value::Null)
}

/// `AddEffectivePermissionSet(RoleId)` narrows the session to the assigned sets (enforced on BC: corpus
/// permissions/TestPermissionsMockEnforcement.al:28-47). Permissions are not enforced here (ADR-027), so a test that
/// lowers them is not run, unless `--ignore-permission-mock` asks to run it anyway. The sets BC's test runner assigns before each test method (Tests-TestLibraries codeunit 130302
/// "Set State Test Run", every Microsoft test that does not disable TestPermissions) are assumed to grant all the test
/// does (`runner-permission-sets-grant-all`): those tests pass in Microsoft's CI under them. On BC they do not grant
/// everything: they grant what D365 Full Access, D365 Basic and LOCAL name, plus Execute on every object (All Objects);
/// a test app table no set names reads as ReadPermission false and fails with DB:ClientReadDenied, and Tenant Media has
/// only indirect write (permissions/lt-qc-runner-sets/QcRunnerSets.al:1-24, :226-291). The corpus applies these sets only
/// from test code, which is refused above, so no corpus case runs through this path.
fn permission_helper_add_set(s: &mut Session, _: Option<&Rc<DotNetObject>>, _: Args) -> Result<Value, Signal> {
    if s.runner_permission_step {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::RunnerPermissionSetsGrantAll);
        return Ok(Value::Null);
    }
    // `al-runner test --ignore-permission-mock`: the user chose to run such a test with every access granted. Each test
    // that gets here is a flagged pass (`permissions-not-enforced`), never mistaken for a permission check.
    if s.ignore_permission_mock {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PermissionsNotEnforced);
        return Ok(Value::Null);
    }
    Err(Signal::Unsupported(
        "DotNet PermissionTestHelper.AddEffectivePermissionSet lowers the permissions of the test session; \
         permissions are not enforced here (every access is granted, ADR-027), so the test is not run \
         (`--ignore-permission-mock` runs it without permission checks, as a flagged pass)"
            .into(),
    ))
}

/// How AL reads an unset environment variable: a null string (tests/al-language-onprem-dotnet/
/// TestDotNetValueKindsContracts.al:130-131, :181-190 and :438, all with a variable no host sets).
const UNSET_VARIABLE_PIN: &str = "tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al:130-131, :181-190, :438 \
    (an unset variable is a null string); owner decision 2026-10-30 (issue 6222): every variable reads as not set";

/// `System.Environment.GetEnvironmentVariable(name)` and `(name, target)`: every variable reads as not set, as on a BC
/// container that sets none (owner decision 2026-10-30, issue 6222). The host's environment is never read, so a run
/// gives the same answer on every machine. The answer is a null string (`UNSET_VARIABLE_PIN`): `= ''` is true, a Text
/// gets '' and a Variant stays empty. Every other member of System.Environment is refused (a registry type serves only
/// its listed members).
fn environment_variable(s: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    if !matches!(
        args,
        [Value::Text(_) | Value::Code(_)] | [Value::Text(_) | Value::Code(_), _]
    ) {
        return Err(Signal::Unsupported(format!(
            "DotNet System.Environment.GetEnvironmentVariable with {} arguments (only (name) and (name, target) are served)",
            args.len()
        )));
    }
    s.dotnet.null_string = true;
    Ok(Value::Null)
}

fn probe(what: &str) -> Pin {
    Pin::Probe(format!("{ENV_PROBE}:{what}"))
}

fn al_runner(what: &str) -> Pin {
    Pin::AlRunner(format!("AL Runner v2 {what}"))
}

/// The shipped types.
pub fn types() -> Vec<RegistryType> {
    let mut all = super::compression::types();
    all.extend(super::last_error::types());
    all.extend(super::blob_stream::types());
    all.extend(super::drawing::types());
    all.extend(super::staged_dir::types());
    all.extend(super::query_metadata::types());
    all.extend(super::generic_dictionary::types());
    all.extend(own_types());
    all
}

fn own_types() -> Vec<RegistryType> {
    let cfg = |name: &str, f: MemberFn| member(name, Pin::Probe(CFG_PIN.into()), f);
    vec![
        RegistryType {
            clr_name: AL_CONFIG_SETTINGS.into(),
            members: vec![
                member(
                    "Instance",
                    al_runner(
                        "unpatched in AlRunner/ (ALConfigSettings.Instance() is a plain constructor call in the \
                         Nav.Types of the oracle build; the settings it reads are the members below)",
                    ),
                    al_config_instance,
                ),
                cfg("EnableSaasExtensionInstallConfigSetting", save_ext_install),
                cfg("IsSaasExcelAddinEnabled", excel_addin),
                cfg("EnableEntraGroupsOnPrem", entra_groups),
                cfg("ApiServicesEnabled", api_services),
                cfg("ApiSubscriptionsEnabled", api_subscriptions),
                cfg("ApiSubscriptionSendingNotificationTimeout", api_sending_timeout),
                cfg("ApiSubscriptionMaxNumberOfNotifications", api_max_notifications),
                cfg("ApiSubscriptionDelayTime", api_delay),
                cfg("TestAutomationEnabled", test_automation),
                cfg("UsePermissionsFromExtensions", permission_sets_from_extensions),
                cfg("IsSaaS", membership_entitlement),
            ],
        },
        RegistryType {
            clr_name: ENVIRONMENT.into(),
            members: vec![member(
                "GetEnvironmentVariable",
                Pin::Corpus(UNSET_VARIABLE_PIN.into()),
                environment_variable,
            )],
        },
        RegistryType {
            clr_name: TENANT_SETTINGS_HELPER.into(),
            members: vec![
                member("IsSandbox", probe("156 (IsSandbox true, IsSaaS true)"), is_sandbox),
                member("IsProduction", probe("157 (IsProduction false)"), is_production),
                member(
                    "SetTestTenantEnvironmentType",
                    Pin::Corpus(
                        "tests/al-language-onprem/session/PVEnvironmentTestability.al:25-60 (staging 8217373)".into(),
                    ),
                    set_test_tenant_environment_type,
                ),
                member(
                    "GetEnvironmentName",
                    probe("180 ('Sandbox' through the wrapper; the raw name is not observable)"),
                    environment_name,
                ),
                member("GetApplicationFamily", probe("181 ('W1')"), application_family),
                member_ref(
                    "TryGetStringTenantSetting",
                    probe("209-212 (TENANTID 'default', DISPLAYNAME '', AADTENANTID 'common')"),
                    try_get_string_tenant_setting,
                ),
                member(
                    "GetLinkedPowerPlatformEnvironmentId",
                    probe("190 ('')"),
                    power_platform_environment_id,
                ),
                member(
                    "GetEnvironmentApplicationSetting",
                    al_runner(
                        "AlRunner/Patches/MetadataPatches.cs:778-835 (empty EnvironmentSettings: every name gives '')",
                    ),
                    environment_application_setting,
                ),
            ],
        },
        RegistryType {
            clr_name: USER_ACCOUNT_HELPER.into(),
            members: vec![
                member(
                    "IsAzure",
                    probe("164-174 (IsSaaSInfrastructure true on 27 and 28, false from 29)"),
                    is_azure,
                ),
                member(
                    "IsUserDelegatedAdmin",
                    Pin::Corpus("al-language-onprem-dotnet/lt-al-dotnet/AlUserAccountHelper.al:62".into()),
                    is_user_delegated_admin,
                ),
                member(
                    "IsDelegatedUser",
                    Pin::Corpus("al-language-onprem-dotnet/lt-al-dotnet/AlUserAccountHelper.al:61".into()),
                    is_delegated_user,
                ),
                member(
                    "IsUserDelegatedHelpdesk",
                    Pin::Corpus("al-language-onprem-dotnet/lt-al-dotnet/AlUserAccountHelper.al:63".into()),
                    is_user_delegated_helpdesk,
                ),
                member(
                    "IsPermissionSetAssigned",
                    Pin::Corpus("platform/lt-outside-platform/RvPermissionSetAssigned.al:74-200".into()),
                    is_permission_set_assigned,
                ),
                member(
                    "GetEffectivePermissionForObject",
                    Pin::Corpus("session/TestSessionCompanyInfoExperienceClose.al:62-90".into()),
                    effective_permission,
                ),
            ],
        },
        RegistryType {
            clr_name: GRAPH_QUERY.into(),
            members: {
                let g = |name: &str, f: MemberFn| member(name, Pin::Probe(GRAPH_PIN.into()), f);
                vec![
                    RegistryMember {
                        kind: MemberKind::Construct,
                        ..g("GraphQuery", graph_query_new)
                    },
                    g("IsM365CollaborationEnabled", graph_collaboration),
                    g("GetEnvironmentDirectoryGroup", graph_directory_group),
                    g("GetGroupPage", graph_group_page),
                    g("GetGroupInfo", graph_group_info),
                    g("FindFirstGroupInfoForDisplayName", graph_first_group),
                    g("GetUser", graph_user),
                    member(
                        "GetTenantDetail",
                        Pin::Corpus(TENANT_INFO_PIN.into()),
                        graph_tenant_detail,
                    ),
                    member(
                        "GetCurrentUser",
                        Pin::Probe(
                            "probe tests/al-language-onprem/session/PZAzureAdUser.al (corpus branch al-runtime-rs-staging, OnPrem codeunit 61296)"
                                .into(),
                        ),
                        graph_current_user,
                    ),
                ]
            },
        },
        RegistryType {
            clr_name: TENANT_INFO.into(),
            members: ["InitialDomain", "CountryLetterCode", "PreferredLanguage"]
                .into_iter()
                .map(|n| member(n, Pin::Corpus(TENANT_INFO_PIN.into()), tenant_info_empty_text))
                .collect(),
        },
        RegistryType {
            clr_name: PERMISSION_TEST_HELPER.into(),
            members: {
                let m = |name: &str, f: MemberFn| member(name, Pin::Corpus(MOCK_LIFECYCLE_PIN.into()), f);
                vec![
                    RegistryMember {
                        kind: MemberKind::Construct,
                        ..m("PermissionTestHelper", permission_helper_new)
                    },
                    m("Clear", permission_helper_clear),
                    m("Dispose", permission_helper_dispose),
                    member(
                        "AddEffectivePermissionSet",
                        Pin::Corpus(
                            "permissions/TestPermissionsMockEnforcement.al:28-47 (the assigned set is enforced)".into(),
                        ),
                        permission_helper_add_set,
                    ),
                ]
            },
        },
        RegistryType {
            clr_name: COPILOT_FUNCTIONS.into(),
            members: vec![member(
                "IsPlatformAuthorizationConfigured",
                Pin::Corpus(
                    "platform/lt-ri-aoai/RiAoaiAuthorization.al:44-94 (false for the caller publisher 'AL Language')"
                        .into(),
                ),
                copilot_platform_authorization,
            )],
        },
        RegistryType {
            clr_name: AKV_PROVIDER.into(),
            members: vec![
                RegistryMember {
                    kind: MemberKind::Construct,
                    ..member("AzureKeyVaultAppSecretProvider", Pin::Corpus(AKV_PIN.into()), akv_new)
                },
                member(
                    "InitializeFromCallingAppExceptSystemApplication",
                    Pin::Corpus(AKV_PIN.into()),
                    akv_initialize,
                ),
                member("TryGetAzureKeyVaultSecret", Pin::Corpus(AKV_PIN.into()), akv_try_get),
            ],
        },
        RegistryType {
            clr_name: TEST_EXECUTION.into(),
            members: vec![member(
                "IsInTestMode",
                probe("189 (CanStartSession false behind IsInTestMode true)"),
                is_in_test_mode,
            )],
        },
    ]
}
