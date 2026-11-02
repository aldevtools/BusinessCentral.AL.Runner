use super::*;

/// `Database.SID(UserName)`: a Windows-style security identifier as Text. Probed on BC 27.5 (corpus
/// branch al-runtime-rs-staging, ae3fe88, tests/al-language-onprem/session/PXDatabaseSid.al:31-46):
/// `S-1-5-21-<a>-<b>-<c>-<rid>`, eight dash-separated parts, all but the first numeric (each read by
/// `Evaluate` into an Integer), the same for the same name, not the text of `UserSecurityId()`, and
/// a SID of that shape for a name no user has. The numbers are the machine's and are not pinned:
/// the domain part is fixed and the rid is a stable hash of the name (docs/probes-needed.md).
/// Without a name, which user is meant is not pinned: refused.
pub(super) fn database_sid(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let Some(name) = args.first().and_then(text_of) else {
        return unsupported("Database.SID without a user name (whose SID it answers is not pinned)");
    };
    crate::assumptions::assume(crate::assumptions::Assumption::DatabaseSidNumbers);
    // FNV-1a over the name's UTF-8 bytes: the same on every platform and run.
    let hash = name.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    let rid = 1000 + hash % (i32::MAX as u64 - 1000);
    Ok(Value::Text(
        format!("S-1-5-21-1004336348-1177238915-682003330-{rid}").into(),
    ))
}

/// `Session.LogMessage`, `Session.LogSecurityAudit` and `Session.LogAuditMessage`: telemetry and the security audit log have
/// no effect AL code can see in a test, so the call returns (session/TestSessionExtended.al:76-89
/// only needs LogMessage callable; session/TestEnvironmentUserTenantContracts.al:315-374 runs
/// Change Log Management, which logs a security audit entry, and reads only the change log).
pub(super) fn log_message(_: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    Ok(Value::Null)
}

/// `Session.LogAuditMessage(Description, Result, Category, Operation, OperationResult [, Dimensions])`: the audit log has no effect AL
/// code can see, but an Operation outside the members of AuditMessageOperation (0 to 7, as the error text lists them) raises
/// (al-language-onprem/lt-system-database-onprem/PLogAuditMessage.al:P109_LogAuditMessage_Overload1 `negative`, `large`).
pub(super) fn log_audit_message(c: &mut dyn Ctx, r: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    if let Some(Value::Integer(op)) = args.get(3) {
        if !(0..=7).contains(op) {
            return raise(
                format!(
                    "The AuditMessageOperation value '{op}' is invalid. Please use one the following values: 0 = AdministeredEnvironment, 1 = ConfiguredExtension, 2 = AdministeredUser, 3 = AdministeredCompany, 4 = ConfiguredIntegration, 5 = ConfiguredCopilot, 6 = ConfiguredCloudMigration, 7 = AdministeredReport."
                ),
                "InvalidValue",
            );
        }
    }
    log_message(c, r, args)
}

/// `Hyperlink(Url)` asks the client to open a page. A bound `[HyperlinkHandler]` gets the URL, also an empty one or a
/// mailto link (handlers/W3Hyperlink.al:24-63); without one the platform raises `Unhandled UI: Hyperlink <url>`
/// (record/ProbeClearAllSetKeyHyperlink.al:322-338, handlers/W3Hyperlink.al:65-83); a routine that only mentions the call
/// still runs.
pub(super) fn hyperlink(c: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let url = text_arg(args, 0)?;
    // Install code has no client: Hyperlink does nothing there (install/lt-outside-install-ui/RlInstallUi.al:224).
    if c.install_without_client() {
        return Ok(Value::Null);
    }
    if c.ui_hyperlink(url.to_string())? {
        return Ok(Value::Null);
    }
    Err(Signal::Error(Box::new(AlError {
        message: format!("Unhandled UI: Hyperlink {url}"),
        code: "MissingUIHandler".into(),
        call_stack: String::new(),
        ..Default::default()
    })))
}

/// `StartSession`, every overload. While an app installs it starts nothing, leaves `SessionId` alone
/// and answers false (or raises nothing when the result is ignored): install/TestInstallStartSession_Tests.al,
/// ALSession.ALStartSessionAsyncImpl. In a test run under TestIsolation = Codeunit it raises the
/// refusal below, also for the overload with a record, and `SessionId` stays 0
/// (session/TestStartSessionRecord.al:StartSession_FromATestCodeunit_IsRefusedUnderCodeunitIsolation).
/// Under Disabled test isolation the session runs to its end inside the call (`Ctx::start_background_session`).
/// Anywhere else a session would start, which this runtime does not do: refused.
pub(super) fn start_session(c: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    start_session_of(c, args, false)
}

/// `StartSession(var SessionId, CodeunitId, Company, var Record, Timeout)`: as [`start_session`], with the Timeout handed
/// to `Ctx::start_background_session_timeout`.
pub(super) fn start_session_timeout(
    c: &mut dyn Ctx,
    _: Option<&mut Value>,
    args: &mut [Value],
) -> Result<Value, Signal> {
    start_session_of(c, args, true)
}

/// `StartSession(var SessionId, CodeunitId, Timeout [, Company [, var Record]])`: no probe measures this overload under
/// Disabled isolation, so it is refused where a session would start.
pub(super) fn start_session_timeout_first(
    c: &mut dyn Ctx,
    _: Option<&mut Value>,
    args: &mut [Value],
) -> Result<Value, Signal> {
    if !c.install_scope() && c.in_test_run() && c.hub_disabled() {
        return unsupported(
            "StartSession with the Timeout before the Company under Disabled test isolation (not pinned)",
        );
    }
    start_session_of(c, args, true)
}

pub(super) fn start_session_of(c: &mut dyn Ctx, args: &mut [Value], timeout: bool) -> Result<Value, Signal> {
    if c.install_scope() {
        return Ok(if c.ret_used() {
            Value::Boolean(false)
        } else {
            Value::Null
        });
    }
    // Under Disabled isolation BC starts the session, it ends and its committed writes are seen: on the TestRunnerHub under
    // RequiredTestIsolation = Disabled (codeunit/lt-qu-commit-isolation/RqIsolationModes.al:101-128, :585-614 RQ_D03,
    // RQ_D04 on BC 28 and later: start_ok, a positive id, session_ended, session_row) and under a test runner codeunit's
    // TestIsolation = Disabled or none (tests/al-language-isolation-probe/src/SdIsolStartSession.Codeunit.al:22-30,
    // :38-63). The session runs to its end inside the call (`Ctx::start_background_session`, which refuses what no probe
    // measures). Under Function it fails as under Codeunit (RqIsolationModes.al:454-462; SdIsolStartSession.Codeunit.al:20-21).
    if c.in_test_run() && c.hub_disabled() {
        // SessionId, CodeunitId, [Company, [Record]] and the overloads with a Timeout after the Record. A session that
        // ends before its Timeout runs as one without; one that would still run at it is refused
        // (`Ctx::start_background_session_timeout`).
        let timeout_ms = if timeout {
            match args.get(4) {
                Some(Value::Integer(n)) if *n > 0 => Some(i64::from(*n)),
                Some(Value::BigInteger(n)) if *n > 0 => Some(*n),
                Some(Value::Duration(d)) if *d > 0 => Some(*d),
                _ => {
                    return unsupported(
                        "StartSession with a Timeout that is not a positive number of milliseconds (not pinned)",
                    );
                }
            }
        } else {
            None
        };
        let Some(Value::Integer(codeunit)) = args.get(1).cloned() else {
            return unsupported("StartSession with a codeunit id that is not an integer");
        };
        let company = match args.get(2) {
            None => None,
            Some(_) => Some(text_arg(args, 2)?.to_string()),
        };
        let record = args.get(3).cloned();
        let session = match timeout_ms {
            None => c.start_background_session(codeunit, company, record)?,
            Some(ms) => c.start_background_session_timeout(codeunit, company, record, ms)?,
        };
        if let Some(slot) = args.first_mut() {
            *slot = Value::Integer(session);
        }
        return Ok(if c.ret_used() {
            Value::Boolean(true)
        } else {
            Value::Null
        });
    }
    if c.in_test_run() {
        return Err(Signal::Error(Box::new(AlError {
            message: "Sessions can only be started in tests that are run by a TestRunner that has TestIsolation set to Disabled."
                .into(),
            code: "TestStartSessionNotAllowed".into(),
            call_stack: String::new(), ..Default::default() })));
    }
    unsupported("StartSession outside an install trigger and a test run (a second session is not modelled)")
}

pub(super) fn context_value(c: &dyn Ctx) -> Value {
    let members: std::sync::Arc<[String]> = crate::sysenum::option_members("ExecutionContext")
        .expect("a system option")
        .into();
    // Install inside an install codeunit, Normal anywhere else (install/TestInstallEnvVisible_Tests.al:90-127).
    Value::Option(i32::from(c.install_scope()), members)
}

pub(super) fn execution_context(c: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    Ok(context_value(c))
}

/// Of this app (no argument) or of an app of the workspace; another app's context while an
/// installer runs is not pinned.
pub(super) fn module_execution_context(
    c: &mut dyn Ctx,
    _: Option<&mut Value>,
    a: &mut [Value],
) -> Result<Value, Signal> {
    if let Some(id) = a.first() {
        if c.install_scope() {
            return unsupported("GetModuleExecutionContext(AppId) inside install code (not pinned)");
        }
        let Value::Guid(g) = id else {
            return unsupported(&format!("GetModuleExecutionContext with {}", id.describe()));
        };
        if !matches!(c.module_by_id(g), ModuleLookup::Found(_)) {
            return unsupported("GetModuleExecutionContext of an app the workspace does not hold (not pinned)");
        }
    }
    Ok(context_value(c))
}
