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
    AL_CONFIG_SETTINGS, GRAPH_QUERY, PERMISSION_TEST_HELPER, TENANT_INFO, TENANT_SETTINGS_HELPER, TEST_EXECUTION,
    USER_ACCOUNT_HELPER,
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

/// `NavUserAccountHelper.IsUserDelegatedAdmin()`: false. The session user is not a delegated user
/// (corpus `session/TestEnvironmentUserTenantContracts.al:245`, `IsUserDelegated` false) and a
/// delegated admin is a delegated user. The member itself is in no corpus test (docs/probes-needed.md).
fn is_user_delegated_admin(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "NavUserAccountHelper.IsUserDelegatedAdmin")?;
    Ok(Value::Boolean(false))
}

/// `NavUserAccountHelper.IsDelegatedUser()`: false, on the same ground as `IsUserDelegatedAdmin` (the session user is not a
/// delegated user, `session/TestEnvironmentUserTenantContracts.al:245`). The member itself is in no corpus test: the
/// owner's exception of 2026-10-15 applies (165 Microsoft tests reach it through `Azure AD Graph User Impl.IsUserDelegated`),
/// so a test that passes through it is a flagged pass (assumption nav-user-account-helper-is-delegated-user).
fn is_delegated_user(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "NavUserAccountHelper.IsDelegatedUser")?;
    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NavUserAccountHelperIsDelegatedUser);
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

/// `NavTestExecution.IsInTestMode()`: true while a test codeunit runs. The probe's
/// `Environment Information.CanStartSession` is false, which is `TaskScheduler.CanCreateTask`
/// (false, also probed) behind this being true.
fn is_in_test_mode(s: &mut Session, _: Option<&Rc<DotNetObject>>, args: Args) -> Result<Value, Signal> {
    no_args(args, "NavTestExecution.IsInTestMode")?;
    Ok(Value::Boolean(s.in_test_codeunit))
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
/// lowers them is not run. The sets BC's test runner assigns before each test method (Tests-TestLibraries codeunit 130302
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
    Err(Signal::Unsupported(
        "DotNet PermissionTestHelper.AddEffectivePermissionSet lowers the permissions of the test session; \
         permissions are not enforced here (every access is granted, ADR-027), so the test is not run"
            .into(),
    ))
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
    all.extend(super::drawing::types());
    all.extend(super::staged_dir::types());
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
                    Pin::Corpus(
                        "session/TestEnvironmentUserTenantContracts.al:245 (IsUserDelegated false; a delegated admin is a delegated user)"
                            .into(),
                    ),
                    is_user_delegated_admin,
                ),
                member(
                    "IsDelegatedUser",
                    Pin::Corpus("session/TestEnvironmentUserTenantContracts.al:245 (inferred: IsUserDelegated false; the member itself is unprobed, assumption nav-user-account-helper-is-delegated-user)".into()),
                    is_delegated_user,
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
                        Pin::Corpus("permissions/TestPermissionsMockEnforcement.al:28-47 (the assigned set is enforced)".into()),
                        permission_helper_add_set,
                    ),
                ]
            },
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
