//! ADR-014 in the VM, with a fake host that records every request: call order (registry, host,
//! refusal), release queue, marshalling, errors, policy. The real sidecar is exercised by
//! `al-test-cli/tests/dotnet_sidecar.rs`.

use al_builtins::native::Signal;
use al_builtins::value::{DotNetObject, Value};
use al_host::dotnet::*;
use al_sema::workspace::AppSource;
use al_vm::dotnet::policy::Policy;
use al_vm::dotnet::registry::{MemberKind, Pin, Registry, RegistryMember, RegistryType};
use al_vm::dotnet::{DotNetConfig, UNAVAILABLE};
use al_vm::{Engine, Session};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
enum Req {
    Open,
    Construct(String, Vec<DotNetArg>),
    Invoke(DotNetTarget, String, Vec<DotNetArg>),
    Get(DotNetTarget, String),
    Set(DotNetTarget, String, DotNetValue),
    Release(Vec<u64>),
    StreamWrite(u64, Vec<u8>),
    Close,
}

type Script = Arc<dyn Fn(&Req) -> Result<DotNetReply, DotNetFailure> + Send + Sync>;

#[derive(Clone)]
struct Fake {
    log: Arc<Mutex<Vec<Req>>>,
    script: Script,
    avail: Result<(), DotNetUnavailable>,
}

impl Fake {
    fn new(script: impl Fn(&Req) -> Result<DotNetReply, DotNetFailure> + Send + Sync + 'static) -> Fake {
        Fake {
            log: Default::default(),
            script: Arc::new(script),
            avail: Ok(()),
        }
    }
    fn log(&self) -> Vec<Req> {
        self.log.lock().unwrap().clone()
    }
    /// The requests that are not `Open`/`Release`/`Close` bookkeeping.
    fn calls(&self) -> Vec<Req> {
        self.log()
            .into_iter()
            .filter(|r| !matches!(r, Req::Open | Req::Release(_) | Req::Close))
            .collect()
    }
}

struct FakeSession(Fake);

impl FakeSession {
    fn go(&mut self, r: Req) -> Result<DotNetReply, DotNetFailure> {
        self.0.log.lock().unwrap().push(r.clone());
        (self.0.script)(&r)
    }
}

impl DotNetSession for FakeSession {
    fn construct(&mut self, ty: &DotNetType, args: Vec<DotNetArg>) -> Result<DotNetReply, DotNetFailure> {
        self.go(Req::Construct(ty.name.clone(), args))
    }
    fn invoke(&mut self, t: DotNetTarget, m: &str, a: Vec<DotNetArg>) -> Result<DotNetReply, DotNetFailure> {
        self.go(Req::Invoke(t, m.into(), a))
    }
    fn get_property(&mut self, t: DotNetTarget, n: &str) -> Result<DotNetReply, DotNetFailure> {
        self.go(Req::Get(t, n.into()))
    }
    fn set_property(&mut self, t: DotNetTarget, n: &str, v: DotNetValue) -> Result<DotNetReply, DotNetFailure> {
        self.go(Req::Set(t, n.into(), v))
    }
    fn stream_write(&mut self, handle: DotNetHandle, bytes: &[u8]) -> Result<(), DotNetFailure> {
        self.0
            .log
            .lock()
            .unwrap()
            .push(Req::StreamWrite(handle, bytes.to_vec()));
        Ok(())
    }
    fn release(&mut self, handles: &[DotNetHandle]) {
        self.0.log.lock().unwrap().push(Req::Release(handles.to_vec()));
    }
    fn close(self: Box<Self>) {
        self.0.log.lock().unwrap().push(Req::Close);
    }
}

impl DotNetHost for Fake {
    fn availability(&self) -> Result<(), DotNetUnavailable> {
        self.avail.clone()
    }
    fn open_session(&self, _: &DotNetSessionConfig) -> Box<dyn DotNetSession> {
        self.log.lock().unwrap().push(Req::Open);
        Box::new(FakeSession(self.clone()))
    }
}

fn reply(v: DotNetValue) -> Result<DotNetReply, DotNetFailure> {
    Ok(DotNetReply::value(v))
}

fn handle(h: u64, ty: &str) -> DotNetValue {
    DotNetValue::Handle {
        handle: h,
        ty: ty.into(),
    }
}

const DOTNET: &str = "dotnet { assembly(\"netstandard\") {
    type(\"System.Convert\"; \"Convert\") { }
    type(\"System.IO.MemoryStream\"; \"MemoryStream\") { }
    type(\"System.Text.Encoding\"; \"Encoding\") { }
    type(\"System.DateTime\"; \"ClrDateTime\") { }
    type(\"System.Byte\"; \"ClrByte\") { }
    type(\"System.IO.File\"; \"File\") { }
    type(\"System.IO.Directory\"; \"Directory\") { }
    type(\"System.IO.DirectoryInfo\"; \"DirectoryInfo\") { }
    type(\"System.IO.FileInfo\"; \"FileInfo\") { }
    type(\"System.Collections.IEnumerator\"; \"IEnumerator\") { }
    type(\"System.Array\"; \"Array\") { }
    type(\"Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper\"; \"NavTenantSettingsHelper\") { }
    type(\"System.Base64FormattingOptions\"; \"Base64FormattingOptions\") { }
    type(\"Microsoft.Dynamics.Nav.Types.Base64UrlHelper\"; \"Base64UrlHelper\") { }
    type(\"Microsoft.Dynamics.Nav.Types.ALConfigSettings\"; \"ALConfigSettings\") { }
    type(\"Microsoft.Dynamics.Nav.NavUserAccount.NavUserAccountHelper\"; \"NavUserAccountHelper\") { }
    type(\"Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper\"; \"PermissionTestHelper\") { }
    type(\"Microsoft.Dynamics.Nav.Runtime.NavTestExecution\"; \"NavTestExecution\") { }
    type(\"Microsoft.Dynamics.Nav.Service.CopilotApi.AL.ALCopilotFunctions\"; \"ALCopilotFunctions\") { }
    type(\"Microsoft.Dynamics.Nav.AzureADGraphClient.GraphQuery\"; \"GraphQuery\") { }
    type(\"Microsoft.Dynamics.Nav.LicensingService.Model.UserInfo\"; \"UserInfo\") { }
    type(\"Microsoft.Dynamics.Nav.LicensingService.Model.TenantInfo\"; \"TenantInfo\") { }
  }
  assembly(\"Microsoft.Exchange.WebServices\") { type(\"Microsoft.Exchange.WebServices.Data.ExchangeService\"; \"ExchangeService\") { } }
}\n";

const TABLE: &str = r#"
table 60008 "ALT Blob"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(3; Data; Blob) { }
    }
    keys { key(PK; "Code") { } }
}
"#;

struct Ran {
    result: Result<Value, Signal>,
    /// Kept so the session (and its host session) lives until the `Ran` is dropped.
    #[allow(dead_code)]
    session: Option<Session>,
}

fn compile(
    vars: &str,
    body: &str,
    ret: &str,
) -> (std::sync::Arc<al_ir::Program>, Vec<Vec<Option<String>>>, usize, usize) {
    let src = format!("{DOTNET}{TABLE}\ncodeunit 50100 C {{ procedure F(){ret} var {vars} begin {body} end; }}");
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src)],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    (l.program, blocked, oi, ri)
}

fn run_with(cfg: DotNetConfig, vars: &str, body: &str, ret: &str) -> Ran {
    run_setup(cfg, vars, body, ret, |_| {})
}

/// As `run_with`, after `setup` has changed the session (for example `in_test_codeunit`).
fn run_setup(cfg: DotNetConfig, vars: &str, body: &str, ret: &str, setup: impl FnOnce(&mut Session)) -> Ran {
    let (program, blocked, oi, ri) = compile(vars, body, ret);
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.get(ri)) {
        return Ran {
            result: Err(Signal::Unsupported(r.clone())),
            session: None,
        };
    }
    let engine = Engine::with_dotnet(program, blocked, cfg);
    let mut s = Session::new(engine);
    setup(&mut s);
    let inst = s.instance(oi);
    let result = s.call(oi, ri, inst, vec![]).map(|(v, _)| v);
    Ran {
        result,
        session: Some(s),
    }
}

/// `run` on an engine that runs as `bc` (a BC version such as "28.5"; `None` is the newest).
fn run_as(bc: Option<&str>, vars: &str, body: &str, ret: &str) -> Ran {
    let (program, blocked, oi, ri) = compile(vars, body, ret);
    let target = bc.map(|v| al_version::Target::for_bc(al_version::BcVersion::parse(v).unwrap()));
    let engine = Engine::with_config(program, blocked, target, cfg(&no_host()));
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    let result = s.call(oi, ri, inst, vec![]).map(|(v, _)| v);
    Ran {
        result,
        session: Some(s),
    }
}

fn cfg(host: &Fake) -> DotNetConfig {
    DotNetConfig {
        host: Arc::new(host.clone()),
        registry: Arc::new(Registry::builtin()),
        policy: Arc::new(Policy::builtin()),
    }
}

fn run(host: &Fake, vars: &str, body: &str, ret: &str) -> Ran {
    run_with(cfg(host), vars, body, ret)
}

fn unsupported(r: &Ran) -> String {
    match &r.result {
        Err(Signal::Unsupported(s)) => s.clone(),
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

fn text(r: Ran) -> String {
    match r.result {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

// ---- call order -------------------------------------------------------------------------

fn convert_script() -> Fake {
    Fake::new(|r| match r {
        Req::Invoke(DotNetTarget::Static(t), m, a) if t.name == "System.Convert" && m == "ToBase64String" => {
            let DotNetValue::Str(s) = &a[0].value else {
                panic!("{a:?}")
            };
            reply(DotNetValue::Str(format!("b64({s})")))
        }
        other => panic!("unexpected request {other:?}"),
    })
}

#[test]
fn a_call_on_the_declared_type_reaches_the_host_with_the_clr_type_and_member() {
    let host = convert_script();
    let r = run(&host, "C: DotNet Convert;", "exit(C.ToBase64String('hi'));", ": Text");
    assert_eq!(text(r), "b64(hi)");
    let log = host.log();
    assert_eq!(log[0], Req::Open, "the session starts at the first call");
    assert!(
        matches!(&log[1], Req::Invoke(DotNetTarget::Static(t), m, _) if t.name == "System.Convert" && t.assembly == "netstandard" && m == "ToBase64String")
    );
    assert_eq!(
        log.last(),
        Some(&Req::Close),
        "the host session closes with the VM session"
    );
}

#[test]
fn a_run_that_never_calls_dotnet_starts_nothing() {
    let host = convert_script();
    let r = run(
        &host,
        "C: DotNet Convert; I: Integer;",
        "I := 1; exit(Format(I));",
        ": Text",
    );
    assert_eq!(text(r), "1");
    assert_eq!(host.log(), vec![], "no Open, no Close");
}

#[test]
fn without_a_host_the_call_is_refused_naming_the_type() {
    let r = run_with(
        DotNetConfig::default(),
        "C: DotNet Convert;",
        "exit(C.ToBase64String('hi'));",
        ": Text",
    );
    let why = unsupported(&r);
    assert!(why.starts_with(UNAVAILABLE), "{why}");
    assert!(
        why.contains("DotNet 'System.Convert' needs a .NET host; none is configured"),
        "{why}"
    );
}

#[test]
fn an_unavailable_host_refuses_with_its_reason_and_opens_no_session() {
    let mut host = convert_script();
    host.avail = Err(DotNetUnavailable(
        "needs the .NET 8 runtime; found 9.0.15; the sidecar needs Microsoft.NETCore.App 8.x".into(),
    ));
    let r = run(&host, "C: DotNet Convert;", "exit(C.ToBase64String('hi'));", ": Text");
    let why = unsupported(&r);
    assert!(why.starts_with(UNAVAILABLE) && why.contains("found 9.0.15"), "{why}");
    assert_eq!(host.log(), vec![]);
}

fn tenant_registry(hits: &'static std::sync::atomic::AtomicUsize) -> Registry {
    fn is_sandbox(_: &mut Session, _: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
        Ok(Value::Boolean(false))
    }
    let _ = hits;
    Registry::new(vec![RegistryType {
        clr_name: "Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper".into(),
        members: vec![RegistryMember {
            name: "IsSandbox".into(),
            kind: MemberKind::Call,
            pin: Pin::Probe("test".into()),
            call: is_sandbox,
            call_ref: None,
        }],
    }])
    .unwrap()
}

#[test]
fn a_registered_type_is_served_by_the_registry_and_never_reaches_the_host() {
    static HITS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let host = Fake::new(|r| panic!("the host must not be called: {r:?}"));
    let mut c = cfg(&host);
    c.registry = Arc::new(tenant_registry(&HITS));
    let r = run_with(
        c,
        "H: DotNet NavTenantSettingsHelper; B: Boolean;",
        "B := H.IsSandbox(); exit(Format(B));",
        ": Text",
    );
    assert_eq!(text(r), "No");
    assert_eq!(host.log(), vec![], "not even a session was opened");
}

#[test]
fn an_unregistered_member_of_a_registered_type_is_refused_by_name_not_forwarded() {
    static HITS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let host = Fake::new(|r| panic!("the host must not be called: {r:?}"));
    let mut c = cfg(&host);
    c.registry = Arc::new(tenant_registry(&HITS));
    let r = run_with(
        c,
        "H: DotNet NavTenantSettingsHelper; B: Boolean;",
        "B := H.IsProduction(); exit(Format(B));",
        ": Text",
    );
    let why = unsupported(&r);
    assert!(why.contains("NavTenantSettingsHelper.IsProduction"), "{why}");
    assert!(why.contains("never forwarded"), "{why}");
    assert_eq!(host.log(), vec![]);
}

/// The test library's setter reaches `SetTestTenantEnvironmentType`; it changes nothing a test can see
/// (corpus tests/al-language-onprem/session/PVEnvironmentTestability.al, staging 8217373).
#[test]
fn set_test_tenant_environment_type_is_accepted_and_changes_nothing() {
    let host = Fake::new(|r| panic!("the host must not be called: {r:?}"));
    let mut c = cfg(&host);
    c.registry = Arc::new(Registry::builtin());
    let r = run_with(
        c,
        "H: DotNet NavTenantSettingsHelper; Before: Boolean; After: Boolean;",
        "Before := H.IsSandbox(); H.SetTestTenantEnvironmentType(not Before); After := H.IsSandbox(); exit(Format(Before = After));",
        ": Text",
    );
    assert_eq!(text(r), "Yes");
    assert_eq!(host.log(), vec![]);
}

/// `--environment` (owner decision 2026-10-15): the tenant helper answers what the chosen server
/// answers. SaaS is the probed oracle tier (IsSandbox Yes, IsProduction No, name ''); on-prem is
/// IsSandbox No and IsProduction Yes (an unprobed assumption required by Microsoft's codeunit
/// 134614, which expects IsSaaS false until the test library sets it).
#[test]
fn the_tenant_helper_follows_the_environment_of_the_host() {
    let answers = |env: Option<al_host::Environment>| {
        let host = Fake::new(|r| panic!("the host must not be called: {r:?}"));
        let (program, blocked, oi, ri) = compile(
            "H: DotNet NavTenantSettingsHelper;",
            "exit(Format(H.IsSandbox()) + Format(H.IsProduction()));",
            ": Text",
        );
        let engine = Engine::with_dotnet(program, blocked, cfg(&host));
        let mut s = match env {
            None => Session::new(engine),
            Some(e) => Session::with_host(engine, al_host::Host::default().in_environment(e)),
        };
        let inst = s.instance(oi);
        match s.call(oi, ri, inst, vec![]).map(|(v, _)| v) {
            Ok(Value::Text(t)) => t,
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(answers(None), "YesNo", "the default is the probed SaaS-like sandbox");
    assert_eq!(answers(Some(al_host::Environment::Saas)), "YesNo");
    assert_eq!(answers(Some(al_host::Environment::OnPrem)), "NoYes");
}

#[test]
fn a_registry_member_without_a_pin_does_not_build() {
    fn nothing(_: &mut Session, _: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
        Ok(Value::Null)
    }
    let err = Registry::new(vec![RegistryType {
        clr_name: "X".into(),
        members: vec![RegistryMember {
            name: "M".into(),
            kind: MemberKind::Call,
            pin: Pin::Corpus("  ".into()),
            call: nothing,
            call_ref: None,
        }],
    }])
    .err()
    .expect("refused");
    assert!(
        err.contains("X.M") && err.contains("neither a corpus pin nor a probe"),
        "{err}"
    );
}

// ---- ADR-024: the clock and settled operands ---------------------------------------------

#[test]
fn datetime_now_is_the_host_clock_and_needs_no_dotnet_host() {
    let host = Fake::new(|r| panic!("the host must not be called: {r:?}"));
    let r = run(
        &host,
        "D: DotNet ClrDateTime; V: DateTime;",
        "V := D.Now; exit(CurrentDateTime() - V = 1);",
        ": Boolean",
    );
    assert!(matches!(r.result, Ok(Value::Boolean(true))), "{:?}", r.result);
}

#[test]
fn a_settled_integer_takes_part_in_arithmetic_and_a_settled_object_is_refused_with_its_type() {
    let host = Fake::new(|r| match r {
        Req::Invoke(_, m, _) if m == "Size" => reply(DotNetValue::Int32(7)),
        Req::Invoke(_, m, _) if m == "Thing" => reply(handle(5, "Some.Clr.Type")),
        other => panic!("{other:?}"),
    });
    let r = run(&host, "C: DotNet Convert;", "exit(C.Size() - 1);", ": Integer");
    assert!(matches!(r.result, Ok(Value::Integer(6))), "{:?}", r.result);
    let r = run(&host, "C: DotNet Convert;", "exit(C.Thing() + 1);", ": Integer");
    let why = unsupported(&r);
    assert!(why.contains("Some.Clr.Type") && why.contains("operand"), "{why}");
}

#[test]
fn a_null_string_is_empty_a_null_object_is_refused_and_ordering_a_string_is_refused() {
    let host = Fake::new(|r| match r {
        Req::Invoke(_, m, _) if m == "NullText" => reply(DotNetValue::NullString),
        Req::Invoke(_, m, _) if m == "NullThing" => reply(DotNetValue::Null),
        Req::Invoke(_, m, _) if m == "Name" => reply(DotNetValue::Str("Abc".into())),
        other => panic!("{other:?}"),
    });
    let r = run(&host, "C: DotNet Convert;", "exit(C.NullText() + 'x');", ": Text");
    assert!(matches!(&r.result, Ok(Value::Text(t)) if t == "x"), "{:?}", r.result);
    let r = run(&host, "C: DotNet Convert;", "exit(C.NullText() = '');", ": Boolean");
    assert!(matches!(r.result, Ok(Value::Boolean(true))), "{:?}", r.result);
    let r = run(&host, "C: DotNet Convert;", "exit(C.NullThing() + 'x');", ": Text");
    assert!(unsupported(&r).contains("null .NET object"));
    let r = run(&host, "C: DotNet Convert;", "exit(C.Name() = 'abc');", ": Boolean");
    assert!(
        matches!(r.result, Ok(Value::Boolean(false))),
        "case sensitive: {:?}",
        r.result
    );
    let r = run(&host, "C: DotNet Convert;", "exit(C.Name() < 'abc');", ": Boolean");
    assert!(unsupported(&r).contains("not pinned"));
    let r = run(&host, "C: DotNet Convert;", "exit(C.Name() < 'b');", ": Boolean");
    assert!(matches!(r.result, Ok(Value::Boolean(true))), "{:?}", r.result);
}

#[test]
fn copystream_into_a_dotnet_stream_hands_the_unread_bytes_to_the_host_and_ends_the_source() {
    let host = Fake::new(|r| match r {
        Req::Construct(..) => reply(handle(9, "System.IO.MemoryStream")),
        other => panic!("{other:?}"),
    });
    let r = run(
        &host,
        "M: DotNet MemoryStream; B: Record \"ALT Blob\" temporary; O: OutStream; I: InStream;",
        "B.Data.CreateOutStream(O); O.WriteText('abc'); B.Data.CreateInStream(I); M := M.MemoryStream(); CopyStream(M, I); exit(I.EOS());",
        ": Boolean",
    );
    assert!(matches!(r.result, Ok(Value::Boolean(true))), "{:?}", r.result);
    assert!(
        host.log().contains(&Req::StreamWrite(9, b"abc".to_vec())),
        "{:?}",
        host.log()
    );
}

#[test]
fn foreach_over_a_dotnet_value_uses_get_enumerator_move_next_and_current() {
    use std::sync::atomic::{AtomicI32, Ordering};
    let moves = Arc::new(AtomicI32::new(0));
    let m = moves.clone();
    let host = Fake::new(move |r| match r {
        Req::Invoke(DotNetTarget::Instance(1), g, _) if g == "GetEnumerator" => reply(handle(2, "Some.Enumerator")),
        Req::Invoke(DotNetTarget::Instance(2), g, _) if g == "MoveNext" => {
            reply(DotNetValue::Bool(m.fetch_add(1, Ordering::SeqCst) < 2))
        }
        Req::Get(DotNetTarget::Instance(2), g) if g == "Current" => {
            reply(DotNetValue::Int32(m.load(Ordering::SeqCst) * 10))
        }
        Req::Invoke(_, g, _) if g == "List" => reply(handle(1, "Some.Collection")),
        other => panic!("{other:?}"),
    });
    let r = run(
        &host,
        "C: DotNet Convert; N: Integer; T: Integer;",
        "foreach N in C.List() do T += N; exit(T);",
        ": Integer",
    );
    assert!(matches!(r.result, Ok(Value::Integer(30))), "{:?}", r.result);
}

// ---- Clear of a DotNet variable ----------------------------------------------------------

#[test]
fn clear_of_an_unassigned_dotnet_variable_changes_nothing_and_of_an_object_is_refused() {
    let host = Fake::new(|r| match r {
        Req::Construct(..) => reply(handle(4, "System.IO.MemoryStream")),
        other => panic!("{other:?}"),
    });
    let r = run(
        &host,
        "M: DotNet MemoryStream;",
        "Clear(M); exit(IsNull(M));",
        ": Boolean",
    );
    assert!(matches!(r.result, Ok(Value::Boolean(true))), "{:?}", r.result);
    let r = run(
        &host,
        "M: DotNet MemoryStream;",
        "M := M.MemoryStream(); Clear(M); exit(IsNull(M));",
        ": Boolean",
    );
    let why = unsupported(&r);
    assert!(
        why.contains("Clear") && why.contains("System.IO.MemoryStream") && why.contains("not pinned"),
        "{why}"
    );
}

// ---- a primitive held in a DotNet variable -----------------------------------------------

#[test]
fn a_dotnet_variable_can_hold_a_primitive_and_a_member_of_it_is_called_on_the_boxed_value() {
    // NPR BloomFilterTests: `ConversionOfByte255ToDotNet`: an AL Byte stored in a `DotNet Byte` variable.
    let host = Fake::new(|r| match r {
        Req::Get(DotNetTarget::Value(DotNetValue::DateTime(t)), n) if n == "Year" => {
            assert!(*t > 600_000_000_000_000_000, "{t}");
            reply(DotNetValue::Int32(2022))
        }
        other => panic!("{other:?}"),
    });
    let r = run(
        &host,
        "B: DotNet ClrByte; I: Integer;",
        "B := 255; I := B; exit(I);",
        ": Integer",
    );
    assert!(matches!(r.result, Ok(Value::Integer(255))), "{:?}", r.result);
    let r = run(&host, "B: DotNet ClrByte;", "B := 7; exit(Format(B));", ": Text");
    assert_eq!(text(r), "7");
    let r = run(
        &host,
        "D: DotNet ClrDateTime; V: DateTime; Y: Integer;",
        "V := CreateDateTime(DMY2Date(1, 1, 2022), 0T); D := V; Y := D.Year; exit(Y);",
        ": Integer",
    );
    assert!(matches!(r.result, Ok(Value::Integer(2022))), "{:?}", r.result);
}

// ---- policy ------------------------------------------------------------------------------

#[test]
fn policy_refusals_name_the_rule_and_never_reach_the_host() {
    let host = Fake::new(|r| panic!("the host must not be called: {r:?}"));
    let r = run(&host, "F: DotNet File;", "F.Delete('x');", "");
    let why = unsupported(&r);
    assert!(
        why.contains("System.IO.File") && why.contains("refused by policy"),
        "{why}"
    );
    let r = run(&host, "E: DotNet ExchangeService;", "E.Foo();", "");
    let why = unsupported(&r);
    assert!(
        why.contains("Microsoft.Exchange.WebServices") && why.contains("network"),
        "{why}"
    );
    assert_eq!(host.log(), vec![]);
}

#[test]
fn an_unregistered_server_class_is_refused_before_the_host() {
    let host = Fake::new(|r| panic!("the host must not be called: {r:?}"));
    let r = run(&host, "H: DotNet UserInfo;", "H.ToString();", "");
    let why = unsupported(&r);
    assert!(
        why.contains("server class") && why.contains("neither registered nor classified pure"),
        "{why}"
    );
    assert_eq!(host.log(), vec![]);
}

#[test]
fn a_server_class_on_the_pure_list_reaches_the_host() {
    let host = Fake::new(|r| match r {
        Req::Invoke(DotNetTarget::Static(t), m, _)
            if t.name == "Microsoft.Dynamics.Nav.Types.Base64UrlHelper" && m == "EncodeBase64UrlString" =>
        {
            reply(DotNetValue::Str("aGk".into()))
        }
        other => panic!("unexpected {other:?}"),
    });
    let r = run(
        &host,
        "H: DotNet Base64UrlHelper;",
        "exit(H.EncodeBase64UrlString('hi'));",
        ": Text",
    );
    assert_eq!(text(r), "aGk");
}

// ---- lifetime ----------------------------------------------------------------------------

fn stream_script() -> Fake {
    let next = Arc::new(std::sync::atomic::AtomicU64::new(1));
    Fake::new(move |r| match r {
        Req::Construct(t, _) if t == "System.IO.MemoryStream" => reply(handle(
            next.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            "System.IO.MemoryStream",
        )),
        Req::Get(DotNetTarget::Instance(_), n) if n == "Length" => reply(DotNetValue::Int64(3)),
        other => panic!("unexpected request {other:?}"),
    })
}

#[test]
fn a_dropped_object_goes_on_the_release_queue_and_the_queue_flushes_before_the_next_call() {
    let host = stream_script();
    let r = run(
        &host,
        "M: DotNet MemoryStream; L: BigInteger;",
        "M := M.MemoryStream(); M := M.MemoryStream(); L := M.Length;",
        "",
    );
    assert!(r.result.is_ok(), "{:?}", r.result);
    let log = host.log();
    // The first object is dropped by the second assignment. Dropping only queues it; the host
    // hears about it just before the next call.
    assert_eq!(log[1], Req::Construct("System.IO.MemoryStream".into(), vec![]));
    assert_eq!(log[2], Req::Construct("System.IO.MemoryStream".into(), vec![]));
    assert_eq!(log[3], Req::Release(vec![1]));
    assert_eq!(log[4], Req::Get(DotNetTarget::Instance(2), "Length".into()));
}

#[test]
fn assignment_shares_the_reference_and_a_handle_is_released_once_after_the_last_owner_goes() {
    let host = stream_script();
    let r = run(
        &host,
        "M: DotNet MemoryStream; N: DotNet MemoryStream; L: BigInteger;",
        "M := M.MemoryStream(); N := M; M := M.MemoryStream(); L := M.Length; L := N.Length;",
        "",
    );
    assert!(r.result.is_ok(), "{:?}", r.result);
    // N still holds the first object: nothing is released while it does, and both Length reads
    // address the handles of their own variable.
    assert_eq!(
        host.calls(),
        vec![
            Req::Construct("System.IO.MemoryStream".into(), vec![]),
            Req::Construct("System.IO.MemoryStream".into(), vec![]),
            Req::Get(DotNetTarget::Instance(2), "Length".into()),
            Req::Get(DotNetTarget::Instance(1), "Length".into()),
        ]
    );
    assert!(
        !host.log().iter().any(|q| matches!(q, Req::Release(_))),
        "{:?}",
        host.log()
    );
    drop(r);
    let mut released: Vec<u64> = host
        .log()
        .iter()
        .filter_map(|q| if let Req::Release(h) = q { Some(h.clone()) } else { None })
        .flatten()
        .collect();
    released.sort();
    assert_eq!(released, vec![1, 2], "each handle exactly once, at session end");
    assert_eq!(host.log().last(), Some(&Req::Close));
}

#[test]
fn session_end_flushes_the_queue_and_closes_the_host() {
    let host = stream_script();
    let (program, blocked, oi, ri) = compile(
        "M: DotNet MemoryStream; L: BigInteger;",
        "M := M.MemoryStream(); L := M.Length;",
        "",
    );
    let engine = Engine::with_dotnet(program, blocked, cfg(&host));
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).unwrap();
    assert!(!host.log().contains(&Req::Close), "still open while the session lives");
    drop(s);
    let log = host.log();
    // The routine's locals (and the object it held) are gone: the handle is released, then the
    // host session closes.
    assert_eq!(&log[log.len() - 2..], &[Req::Release(vec![1]), Req::Close]);
}

#[test]
fn constructing_property_get_and_a_returned_handle_round_trip() {
    let host = stream_script();
    let r = run(
        &host,
        "M: DotNet MemoryStream; L: BigInteger;",
        "M := M.MemoryStream(); L := M.Length; exit(Format(L));",
        ": Text",
    );
    assert_eq!(text(r), "3");
    assert_eq!(
        host.calls(),
        vec![
            Req::Construct("System.IO.MemoryStream".into(), vec![]),
            Req::Get(DotNetTarget::Instance(1), "Length".into()),
        ]
    );
}

// ---- marshalling (ADR-014 section 4), one row per AL type --------------------------------

fn arg_of(vars: &str, setup: &str) -> Result<DotNetValue, String> {
    let host = Fake::new(|_| reply(DotNetValue::Null));
    let r = run(
        &host,
        &format!("C: DotNet Convert; {vars}"),
        &format!("{setup} C.Take(V);"),
        "",
    );
    match r.result {
        Ok(_) => match host.calls().first() {
            Some(Req::Invoke(DotNetTarget::Static(t), m, a))
                if t.name == "System.Convert" && m == "Take" && a.len() == 1 =>
            {
                Ok(a[0].value.clone())
            }
            other => Err(format!("{other:?}")),
        },
        Err(Signal::Unsupported(s)) => Err(s),
        Err(other) => Err(format!("{other:?}")),
    }
}

#[test]
fn al_values_go_to_dotnet_as_the_marshalling_table_says() {
    // 2024-01-01 is 638396640000000000 ticks after 0001-01-01 in .NET.
    let day = 638_396_640_000_000_000_i64;
    let rows: Vec<(&str, &str, DotNetValue)> = vec![
        ("V: Text;", "V := 'héllo';", DotNetValue::Str("héllo".into())),
        ("V: Code[10];", "V := 'abc';", DotNetValue::Str("ABC".into())),
        ("V: Integer;", "V := -5;", DotNetValue::Int32(-5)),
        ("V: BigInteger;", "V := 5000000000L;", DotNetValue::Int64(5_000_000_000)),
        ("V: Byte;", "V := 200;", DotNetValue::Byte(200)),
        ("V: Char;", "V := 65;", DotNetValue::Char(65)),
        ("V: Boolean;", "V := true;", DotNetValue::Bool(true)),
        ("V: Decimal;", "V := 1.5;", DotNetValue::Decimal("1.5".into())),
        ("V: Date;", "V := 20240101D;", DotNetValue::DateTime(day)),
        (
            "V: DateTime;",
            "Evaluate(V, '2024-01-01T12:00:00Z', 9);",
            DotNetValue::DateTime(day + 432_000_000_000),
        ),
        ("V: Duration;", "V := 1500;", DotNetValue::TimeSpan(15_000_000)),
        (
            "V: Guid;",
            "Evaluate(V, '{01234567-89AB-CDEF-0123-456789ABCDEF}');",
            DotNetValue::Guid([
                0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
            ]),
        ),
        ("V: Option A,B,C;", "V := V::C;", DotNetValue::Int32(2)),
        ("V: DotNet MemoryStream;", "", DotNetValue::Null),
    ];
    for (vars, setup, want) in rows {
        assert_eq!(arg_of(vars, setup), Ok(want), "{vars} {setup}");
    }
}

#[test]
fn values_with_no_pinned_marshalling_are_refused_with_a_reason() {
    for (vars, setup, needle) in [
        ("V: Time;", "V := 120000T;", "Time"),
        ("V: Date;", "", "undefined Date"),
        ("V: DateTime;", "", "undefined Date"),
        ("V: List of [Text];", "", "no marshalling row"),
        ("V: Record \"ALT Blob\";", "", "no marshalling row"),
    ] {
        let why = arg_of(vars, setup).expect_err(vars);
        assert!(why.contains(needle), "{vars}: {why}");
    }
}

fn result_of(reply_value: DotNetValue, var: &str, ret: &str) -> Result<Value, Signal> {
    let host = Fake::new(move |_| reply(reply_value.clone()));
    run(
        &host,
        &format!("C: DotNet Convert; V: {var};"),
        "V := C.Make(); exit(V);",
        ret,
    )
    .result
}

#[test]
fn dotnet_results_come_back_as_the_marshalling_table_says() {
    let day = 638_396_640_000_000_000_i64;
    assert!(matches!(result_of(DotNetValue::Str("x".into()), "Text", ": Text"), Ok(Value::Text(s)) if s == "x"));
    assert!(matches!(
        result_of(DotNetValue::Int32(7), "Integer", ": Integer"),
        Ok(Value::Integer(7))
    ));
    assert!(matches!(
        result_of(DotNetValue::Int64(7), "BigInteger", ": BigInteger"),
        Ok(Value::BigInteger(7))
    ));
    assert!(matches!(
        result_of(DotNetValue::Bool(true), "Boolean", ": Boolean"),
        Ok(Value::Boolean(true))
    ));
    assert!(
        matches!(result_of(DotNetValue::Decimal("2.50".into()), "Decimal", ": Decimal"), Ok(Value::Decimal(d)) if d.to_string() == "2.50")
    );
    assert!(
        matches!(result_of(DotNetValue::Int32(3), "Decimal", ": Decimal"), Ok(Value::Decimal(d)) if d.to_string() == "3")
    );
    // 2024-01-01 12:00 is 45_000 s after midnight; AL stores ms since 0001-01-01 plus one.
    let want_ms = (day + 432_000_000_000) / 10_000 + 1;
    assert!(
        matches!(result_of(DotNetValue::DateTime(day + 432_000_000_000), "DateTime", ": DateTime"), Ok(Value::DateTime(ms)) if ms == want_ms)
    );
    assert!(matches!(
        result_of(DotNetValue::TimeSpan(15_000_000), "Duration", ": Duration"),
        Ok(Value::Duration(1500))
    ));
    assert!(matches!(result_of(DotNetValue::Guid([7; 16]), "Guid", ": Guid"), Ok(Value::Guid(g)) if g == [7; 16]));
    assert!(matches!(
        result_of(DotNetValue::Null, "DotNet MemoryStream", ": Integer"),
        Err(Signal::Unsupported(_)) | Ok(_)
    ));
}

#[test]
fn a_result_of_another_kind_is_not_stored_into_a_typed_variable() {
    // BC's conversion of an Integer result into a Text variable is not pinned.
    let e = result_of(DotNetValue::Int32(7), "Text", ": Text").expect_err("refused");
    let Signal::Unsupported(why) = e else { panic!() };
    assert!(why.contains("Integer") && why.contains("not pinned"), "{why}");
    // A narrowing result (long into Integer) is refused too.
    assert!(result_of(DotNetValue::Int64(7), "Integer", ": Integer").is_err());
}

#[test]
fn an_enum_result_keeps_its_clr_type_and_goes_back_as_an_enum_argument() {
    let host = Fake::new(|r| match r {
        Req::Get(DotNetTarget::Static(t), n)
            if t.name == "System.Base64FormattingOptions" && n == "InsertLineBreaks" =>
        {
            reply(DotNetValue::Enum {
                ty: "System.Base64FormattingOptions".into(),
                name: "InsertLineBreaks".into(),
                ordinal: 1,
            })
        }
        Req::Invoke(_, m, a) if m == "ToBase64String" => {
            assert_eq!(
                a[1].value,
                DotNetValue::Enum {
                    ty: "System.Base64FormattingOptions".into(),
                    name: "InsertLineBreaks".into(),
                    ordinal: 1
                }
            );
            reply(DotNetValue::Str("ok".into()))
        }
        other => panic!("unexpected {other:?}"),
    });
    let r = run(
        &host,
        "C: DotNet Convert; O: DotNet Base64FormattingOptions; V: DotNet Base64FormattingOptions; T: Text;",
        "V := O.InsertLineBreaks; T := C.ToBase64String('x', V); exit(T);",
        ": Text",
    );
    assert_eq!(text(r), "ok");
}

// ---- errors and by-reference arguments ----------------------------------------------------

#[test]
fn a_dotnet_exception_is_unsupported_with_the_clr_type_message_and_inner_exceptions() {
    let host = Fake::new(|_| {
        Err(DotNetFailure::Exception(DotNetException {
            clr_type: "System.FormatException".into(),
            message: "The input is not a valid Base-64 string".into(),
            inner: vec![("System.ArgumentException".into(), "inner text".into())],
        }))
    });
    let r = run(
        &host,
        "C: DotNet Convert; T: Text;",
        "T := C.FromBase64String('not base64!');",
        "",
    );
    let why = unsupported(&r);
    assert!(why.contains("System.Convert.FromBase64String"), "{why}");
    assert!(
        why.contains("System.FormatException: The input is not a valid Base-64 string"),
        "{why}"
    );
    assert!(why.contains("System.ArgumentException: inner text"), "{why}");
    assert!(why.contains("not pinned"), "{why}");
}

// The runtime's .NET host has only the encodings .NET itself ships (no code pages such as 1252). Whether BC's server
// answers Encoding.GetEncoding(1252) with an encoding is not pinned (docs/probes-needed.md, "Encoding.GetEncoding with a
// code page"), so the refusal says that, not the host's NotSupportedException as if BC raised it.
#[test]
fn get_encoding_of_a_code_page_the_host_lacks_is_refused_as_not_pinned() {
    let host = Fake::new(|_| {
        Err(DotNetFailure::Exception(DotNetException {
            clr_type: "System.NotSupportedException".into(),
            message: "No data is available for encoding 1252. For information on defining a custom encoding, see the documentation for the Encoding.RegisterProvider method.".into(),
            inner: vec![],
        }))
    });
    let r = run(
        &host,
        "E: DotNet Encoding; X: DotNet Encoding;",
        "X := E.GetEncoding(1252);",
        "",
    );
    let why = unsupported(&r);
    assert_eq!(
        why,
        "DotNet System.Text.Encoding.GetEncoding: the runtime's .NET host has no code-page encodings (code page 1252); \
         whether Business Central provides them is not pinned (docs/probes-needed.md)"
    );
}

#[test]
fn asserterror_around_a_dotnet_exception_is_not_run_until_the_text_is_pinned() {
    let host = Fake::new(|_| {
        Err(DotNetFailure::Exception(DotNetException {
            clr_type: "System.FormatException".into(),
            message: "bad".into(),
            inner: vec![],
        }))
    });
    let r = run(
        &host,
        "C: DotNet Convert; T: Text;",
        "asserterror T := C.FromBase64String('x'); exit(GetLastErrorText());",
        ": Text",
    );
    assert!(
        unsupported(&r).contains("System.FormatException"),
        "Unsupported passes through asserterror"
    );
}

#[test]
fn a_refusal_from_the_host_names_the_member() {
    let host = Fake::new(|_| Err(DotNetFailure::Refused("member not found".into())));
    let r = run(&host, "C: DotNet Convert;", "C.Nope();", "");
    assert_eq!(unsupported(&r), "DotNet System.Convert.Nope: member not found");
}

#[test]
fn by_reference_results_are_written_back_to_the_variable_arguments() {
    let host = Fake::new(|r| match r {
        Req::Invoke(_, m, a) if m == "TryParse" => {
            assert_eq!(a.iter().map(|x| x.by_ref).collect::<Vec<_>>(), vec![false, true]);
            Ok(DotNetReply {
                value: DotNetValue::Bool(true),
                by_ref: vec![(1, DotNetValue::Int32(42))],
                streams: Default::default(),
            })
        }
        other => panic!("{other:?}"),
    });
    let r = run(
        &host,
        "C: DotNet Convert; N: Integer; B: Boolean;",
        "B := C.TryParse('42', N); exit(Format(N));",
        ": Text",
    );
    assert_eq!(text(r), "42");
}

// ---- streams (ADR-014 section 4) -----------------------------------------------------------

#[test]
fn an_instream_argument_sends_the_unread_bytes_and_the_read_position_follows_what_dotnet_consumed() {
    let host = Fake::new(|r| match r {
        Req::Invoke(_, m, a) if m == "ReadSome" => {
            assert_eq!(a[0].value, DotNetValue::InStream(b"abcdef".to_vec()));
            Ok(DotNetReply {
                value: DotNetValue::Null,
                by_ref: vec![],
                streams: StreamEffects {
                    consumed: vec![(0, 4)],
                    ..Default::default()
                },
            })
        }
        other => panic!("{other:?}"),
    });
    let r = run(
        &host,
        "C: DotNet Convert; B: Record \"ALT Blob\"; O: OutStream; I: InStream; T: Text;",
        "B.Code := 'S'; B.Data.CreateOutStream(O); O.WriteText('abcdef'); B.Insert(); B.Get('S'); B.CalcFields(Data);
         B.Data.CreateInStream(I); C.ReadSome(I); I.ReadText(T); exit(T);",
        ": Text",
    );
    assert_eq!(text(r), "ef", "AL reads on from the byte after the four .NET consumed");
}

#[test]
fn an_outstream_argument_is_bridged_and_the_bytes_dotnet_wrote_are_appended_to_the_blob() {
    let host = Fake::new(|r| match r {
        Req::Invoke(_, m, a) if m == "WriteTo" => {
            assert_eq!(a[0].value, DotNetValue::OutStream);
            Ok(DotNetReply {
                value: DotNetValue::Null,
                by_ref: vec![],
                streams: StreamEffects {
                    bridges: vec![(0, 9)],
                    written: vec![(9, b"from .net".to_vec())],
                    ..Default::default()
                },
            })
        }
        Req::Invoke(_, m, _) if m == "WriteMore" => Ok(DotNetReply {
            value: DotNetValue::Null,
            by_ref: vec![],
            streams: StreamEffects {
                written: vec![(9, b"!".to_vec())],
                ..Default::default()
            },
        }),
        other => panic!("{other:?}"),
    });
    let r = run(
        &host,
        "C: DotNet Convert; B: Record \"ALT Blob\"; O: OutStream; I: InStream; T: Text;",
        "B.Code := 'S'; B.Data.CreateOutStream(O); C.WriteTo(O); C.WriteMore(); B.Insert(); B.Get('S'); B.CalcFields(Data);
         B.Data.CreateInStream(I); I.ReadText(T); exit(T);",
        ": Text",
    );
    assert_eq!(text(r), "from .net!", "a later call can still write into the bridge");
}

// ---- the shipped registry (owner decision 2026-10-14) ------------------------------------

fn no_host() -> Fake {
    Fake::new(|r| panic!("a registered type must never reach the host: {r:?}"))
}

fn run_text(vars: &str, body: &str) -> String {
    let host = no_host();
    let r = run(&host, vars, &format!("exit(Format({body}));"), ": Text");
    let out = text(r);
    assert_eq!(host.log(), vec![], "not even a session was opened");
    out
}

/// Each value, with its source. The ALConfigSettings flags, the environment type, the topology and
/// the application family are measured on real BC 27.5 and later (corpus
/// `session/TestEnvironmentUserTenantContracts.al` and the OnPrem suite's `TestServerSettingContracts.al`). The probe
/// wins over AL Runner v2 where they differ: the oracle tenant is a sandbox on Azure topology.
#[test]
fn the_server_class_members_answer_the_values_of_the_probe_and_al_runner_v2() {
    let cfg = "C: DotNet ALConfigSettings;";
    for (member, want) in [
        ("EnableSaasExtensionInstallConfigSetting", "No"),
        ("IsSaasExcelAddinEnabled", "No"),
        ("EnableEntraGroupsOnPrem", "No"),
        ("ApiServicesEnabled", "Yes"),
        ("ApiSubscriptionsEnabled", "Yes"),
        ("ApiSubscriptionSendingNotificationTimeout", "30000"),
        ("ApiSubscriptionMaxNumberOfNotifications", "1000"),
        ("ApiSubscriptionDelayTime", "30000"),
        ("TestAutomationEnabled", "Yes"),
        ("UsePermissionsFromExtensions", "Yes"),
        ("IsSaaS", "No"),
    ] {
        assert_eq!(run_text(cfg, &format!("C.{member}()")), want, "{member}");
    }
    let t = "T: DotNet NavTenantSettingsHelper;";
    assert_eq!(run_text(t, "T.IsSandbox()"), "Yes");
    assert_eq!(run_text(t, "T.IsProduction()"), "No");
    assert_eq!(run_text(t, "T.GetEnvironmentName()"), "");
    assert_eq!(run_text(t, "T.GetApplicationFamily()"), "W1");
    assert_eq!(run_text(t, "T.GetLinkedPowerPlatformEnvironmentId()"), "");
    assert_eq!(
        run_text(t, "T.GetEnvironmentApplicationSetting('EnableAgentBuilder')"),
        ""
    );
    // IsAzure is the one answer that follows the BC major: see `is_azure_follows_the_bc_major`.
    assert_eq!(
        text(run_as(
            Some("27.5"),
            "U: DotNet NavUserAccountHelper;",
            "exit(Format(U.IsAzure()));",
            ": Text"
        )),
        "Yes"
    );
}

/// Tests-ERM `Azure AD Identity Mgmt.IsUserDelegatedAdmin`: the session user is not delegated (corpus
/// `session/TestEnvironmentUserTenantContracts.al:245`, `IsUserDelegated` false), and a delegated admin is a
/// delegated user, so it is not a delegated admin either.
#[test]
fn the_session_user_is_not_a_delegated_admin() {
    assert_eq!(
        text(run_as(
            Some("27.5"),
            "U: DotNet NavUserAccountHelper;",
            "exit(Format(U.IsUserDelegatedAdmin()));",
            ": Text"
        )),
        "No"
    );
}

/// System Application `Azure AD Graph User Impl.IsUserDelegated` calls `NavUserAccountHelper.IsDelegatedUser()`; 165 Microsoft
/// tests (for example Suggest Price Lines UT T202) need it. The session user is not delegated (assumption
/// nav-user-account-helper-is-delegated-user).
#[test]
fn the_session_user_is_not_a_delegated_user() {
    assert_eq!(
        text(run_as(
            Some("27.5"),
            "U: DotNet NavUserAccountHelper;",
            "exit(Format(U.IsDelegatedUser()));",
            ": Text"
        )),
        "No"
    );
}

/// `Environment Information.IsSaaSInfrastructure` is `NavUserAccountHelper.IsAzure()`: true on the
/// BC 27.5 and 28.5 oracle runs, false on the 29.0 run (corpus
/// `session/TestEnvironmentUserTenantContracts.al:164-174`, `#if BC29PLUS`). No target is the newest.
#[test]
fn is_azure_follows_the_bc_major() {
    let u = "U: DotNet NavUserAccountHelper;";
    for (bc, want) in [
        (Some("27.5"), "Yes"),
        (Some("28.5"), "Yes"),
        (Some("29.0"), "No"),
        (Some("30.0"), "No"),
        (None, "No"),
    ] {
        assert_eq!(
            text(run_as(bc, u, "exit(Format(U.IsAzure()));", ": Text")),
            want,
            "{bc:?}"
        );
    }
}

/// Base App's `Environment Information Impl.` reads these in a test run; `CanStartSession` is
/// false in the probe because `IsInTestMode` is true there.
#[test]
fn is_in_test_mode_follows_the_running_test_codeunit() {
    let vars = "E: DotNet NavTestExecution;";
    let body = "exit(Format(E.IsInTestMode()));";
    assert_eq!(
        text(run_setup(cfg(&no_host()), vars, body, ": Text", |s| s
            .in_test_codeunit =
            true)),
        "Yes"
    );
    assert_eq!(text(run(&no_host(), vars, body, ": Text")), "No");
}

/// `ALCopilotFunctions.IsPlatformAuthorizationConfigured(caller publisher, System Application publisher)`: false for a caller
/// that is not Microsoft (corpus platform/lt-ri-aoai/RiAoaiAuthorization.al:44-94, publisher 'AL Language'). A Microsoft caller
/// is not pinned and is refused, and so is a call with other arguments; nothing reaches the host.
#[test]
fn copilot_platform_authorization_is_not_configured_for_a_partner_caller() {
    let vars = "F: DotNet ALCopilotFunctions;";
    let host = no_host();
    let r = run(
        &host,
        vars,
        "exit(Format(F.IsPlatformAuthorizationConfigured('AL Language', 'Microsoft')));",
        ": Text",
    );
    assert_eq!(text(r), "No");
    let r = run(
        &host,
        vars,
        "exit(Format(F.IsPlatformAuthorizationConfigured('Microsoft', 'Microsoft')));",
        ": Text",
    );
    assert!(unsupported(&r).contains("IsPlatformAuthorizationConfigured"));
    let r = run(
        &host,
        vars,
        "exit(Format(F.IsPlatformAuthorizationConfigured('A')));",
        ": Text",
    );
    assert!(unsupported(&r).contains("IsPlatformAuthorizationConfigured"));
    assert_eq!(host.log(), vec![]);
}

/// A SUPER session on table data: the four data rights, no Execute (corpus
/// `session/TestSessionCompanyInfoExperienceClose.al`). Anything else is refused.
#[test]
fn the_effective_permission_of_the_session_user_on_table_data_is_the_four_data_rights() {
    let vars = "U: DotNet NavUserAccountHelper;";
    let host = no_host();
    let r = run(
        &host,
        vars,
        "exit(U.GetEffectivePermissionForObject(UserSecurityId(), CompanyName(), 0, 79));",
        ": Text",
    );
    assert_eq!(text(r), "1,1,1,1,0");
    let r = run(
        &host,
        vars,
        "exit(U.GetEffectivePermissionForObject(UserSecurityId(), CompanyName(), 5, 79));",
        ": Text",
    );
    assert!(unsupported(&r).contains("other than table data"), "{:?}", r.result);
    let r = run(
        &host,
        vars,
        "exit(U.GetEffectivePermissionForObject(CreateGuid(), CompanyName(), 0, 79));",
        ": Text",
    );
    assert!(unsupported(&r).contains("another user"), "{:?}", r.result);
    assert_eq!(host.log(), vec![]);
}

/// `ServerSettingImpl` does `ALConfigSettings := ALConfigSettings.Instance()` and then calls the
/// settings on the object it got back.
#[test]
fn an_instance_of_a_registered_type_serves_its_members_from_the_registry() {
    let host = no_host();
    let r = run(
        &host,
        "C: DotNet ALConfigSettings; D: DotNet ALConfigSettings;",
        "C := C.Instance(); D := C; exit(Format(D.ApiSubscriptionDelayTime()));",
        ": Text",
    );
    assert_eq!(text(r), "30000");
    assert_eq!(host.log(), vec![]);
}

#[test]
fn registry_results_are_operands() {
    let host = no_host();
    // BC 27.5, where IsAzure is true.
    let r = run_as(
        Some("27.5"),
        "U: DotNet NavUserAccountHelper; C: DotNet ALConfigSettings; N: Integer;",
        "C := C.Instance(); if C.ApiSubscriptionDelayTime() > 1000 then N := 1; \
         if U.IsAzure() and C.ApiServicesEnabled() then N += 10; exit(N);",
        ": Integer",
    );
    assert!(matches!(r.result, Ok(Value::Integer(11))), "{:?}", r.result);
    assert_eq!(host.log(), vec![]);
}

/// session/TestEnvironmentUserTenantContracts.al:TenantAndAzureAd_Answers (:209-212) pins the three
/// settings System Application reads through `TryGetStringTenantSetting(name, var value)`.
#[test]
fn try_get_string_tenant_setting_answers_the_pinned_names_and_refuses_the_others() {
    let host = no_host();
    for (name, want) in [("TENANTID", "default"), ("DISPLAYNAME", ""), ("AADTENANTID", "common")] {
        let r = run(
            &host,
            "T: DotNet NavTenantSettingsHelper; V: Text;",
            &format!("V := 'unset'; if not T.TryGetStringTenantSetting('{name}', V) then exit('false'); exit(V);"),
            ": Text",
        );
        assert_eq!(text(r), want, "{name}");
    }
    let r = run(
        &host,
        "T: DotNet NavTenantSettingsHelper; V: Text;",
        "exit(Format(T.TryGetStringTenantSetting('SOMETHINGELSE', V)));",
        ": Text",
    );
    assert!(unsupported(&r).contains("SOMETHINGELSE"));
    assert_eq!(host.log(), vec![]);
}

/// The rule of ADR-014 section 1 for every shipped type: a member the registry does not
/// implement is refused by name, and nothing a registered type does reaches the host.
#[test]
fn a_registered_type_is_never_forwarded_to_the_host() {
    for (alias, clr) in [
        ("ALConfigSettings", al_builtins::dotnet_sigs::AL_CONFIG_SETTINGS),
        (
            "NavTenantSettingsHelper",
            al_builtins::dotnet_sigs::TENANT_SETTINGS_HELPER,
        ),
        ("NavUserAccountHelper", al_builtins::dotnet_sigs::USER_ACCOUNT_HELPER),
        ("NavTestExecution", al_builtins::dotnet_sigs::TEST_EXECUTION),
    ] {
        let host = no_host();
        let r = run(
            &host,
            &format!("X: DotNet {alias};"),
            "X.NotAMemberOfTheRealClass();",
            "",
        );
        let why = unsupported(&r);
        assert!(
            why.contains(&format!("{clr}.NotAMemberOfTheRealClass")) && why.contains("never forwarded"),
            "{why}"
        );
        assert_eq!(host.log(), vec![], "{alias}");
        // A real member the runtime does not provide is refused the same way, not run on .NET.
    }
    let host = no_host();
    let r = run(
        &host,
        "T: DotNet NavTenantSettingsHelper; S: Text;",
        "S := T.GetRingName();",
        "",
    );
    assert!(unsupported(&r).contains("NavTenantSettingsHelper.GetRingName"));
    assert_eq!(host.log(), vec![]);
}

/// The compiler types a result from `dotnet_sigs`; the registry implements it. The two lists
/// name the same members, and each member returns the kind of value its row declares.
#[test]
fn the_declared_result_types_match_the_registry() {
    use al_builtins::dotnet_sigs::{Ret, SIGS};
    let reg = Registry::builtin();
    for (clr, member, ret) in SIGS {
        let t = reg
            .get(clr)
            .unwrap_or_else(|| panic!("{clr} has signatures but is not registered"));
        assert!(
            t.members.iter().any(|m| m.name == *member),
            "{clr}.{member} has a signature but no registry member"
        );
        let _ = ret;
    }
    for t in reg.types() {
        for m in &t.members {
            assert!(
                al_builtins::dotnet_sigs::lookup(&t.clr_name, &m.name).is_some(),
                "{}.{} is registered but has no declared result type",
                t.clr_name,
                m.name
            );
            assert!(
                matches!(&m.pin, Pin::AlRunner(s) if s.starts_with("AL Runner v2 ") && s.contains("AlRunner/"))
                    || matches!(&m.pin, Pin::Probe(s) if s.contains("probe tests/al-language") && s.contains(".al"))
                    || matches!(&m.pin, Pin::Corpus(s) if s.contains(".al:"))
                    || matches!(&m.pin, Pin::Microsoft(s) if s.contains("O365BrandColorTests.Codeunit.al:")),
                "{}.{}: {:?}",
                t.clr_name,
                m.name,
                m.pin
            );
        }
    }
    // The kind of value each parameterless member returns.
    for (clr, member, ret) in SIGS {
        let t = reg.get(clr).unwrap();
        let m = t.members.iter().find(|m| m.name == *member).unwrap();
        let args: Vec<Value> = match *member {
            "GetEnvironmentApplicationSetting" => vec![Value::Text("x".into())],
            "GetEffectivePermissionForObject" => continue, // needs a Guid; the AL test above runs it
            // Need streams; tests/gzip.rs runs them.
            "GZipStream" | "CopyTo" | "Dispose" => continue,
            // Need other runtime objects; tests/host/bitmap.rs runs them.
            "Bitmap" | "Save" | "FromImage" | "FillEllipse" | "FromHtml" | "SolidBrush" => continue,
            // Need an exception object; tests/host/last_error_object.rs runs them.
            "Message" | "InnerException" | "GetType" | "ToString" | "FullName" | "GetBaseException" | "Equals" => {
                continue;
            }
            "SetTestTenantEnvironmentType" => vec![Value::Boolean(false)],
            "IsPlatformAuthorizationConfigured" => {
                vec![Value::Text("AL Language".into()), Value::Text("Microsoft".into())]
            }
            // These raise by design on a tier without a directory; graph_query_without_a_directory_answers_as_probed runs them.
            "GetGroupPage" | "GetGroupInfo" | "FindFirstGroupInfoForDisplayName" | "GetUser" => continue,
            // Answers a null reference by design; graph_get_current_user_returns_null_without_an_error runs it.
            "GetCurrentUser" | "GetTenantDetail" => continue,
            // Refused by design (it lowers permissions); adding_an_effective_permission_set_is_refused runs it.
            "AddEffectivePermissionSet" => continue,
            // Need a staged folder; directory_info_of_a_folder_staged_for_the_company_build_* run them.
            "DirectoryInfo" | "Exists" | "GetFiles" | "GetEnumerator" | "MoveNext" | "Current" | "Length" => continue,
            _ => vec![],
        };
        let mut s = Session::new(Engine::new(al_ir::Program::default(), vec![]));
        let mut args = args;
        if *member == "TryGetStringTenantSetting" {
            args = vec![Value::Text("TENANTID".into()), Value::Text(String::new().into())];
        }
        let v = match m.call_ref {
            Some(f) => f(&mut s, None, &mut args),
            None => (m.call)(&mut s, None, &args),
        }
        .unwrap_or_else(|e| panic!("{clr}.{member}: {e:?}"));
        let ok = match ret {
            Ret::Boolean => matches!(v, Value::Boolean(_)),
            Ret::Integer => matches!(v, Value::Integer(_)),
            Ret::Text => matches!(v, Value::Text(_)),
            Ret::Object => matches!(v, Value::DotNet(_)),
            Ret::Void => matches!(v, Value::Null),
        };
        assert!(ok, "{clr}.{member} declares {ret:?} but returns {v:?}");
    }
}

// Azure AD Graph on a tier without Entra ID, as probed on BC 27.5 (corpus branch al-runtime-rs-staging,
// tests/al-language-onprem/session/PXAzureAdGraph.al:1-12, 86-109): the GraphQuery object is created, the
// collaboration flag is false and the environment group is empty; the calls that need the directory raise a
// NullReference. The runtime answers all of it itself; nothing goes to .NET.
#[test]
fn graph_query_without_a_directory_answers_as_probed() {
    let host = no_host();
    let r = run(
        &host,
        "G: DotNet GraphQuery; s: Text;",
        "G := G.GraphQuery();
         s := Format(IsNull(G)) + Format(G.IsM365CollaborationEnabled()) + '[' + G.GetEnvironmentDirectoryGroup() + ']';
         asserterror G.GetGroupPage(10); s += '|' + GetLastErrorText() + '#' + GetLastErrorCode();
         asserterror G.GetGroupInfo('id'); s += '|' + GetLastErrorText();
         asserterror G.FindFirstGroupInfoForDisplayName('n'); s += '|' + GetLastErrorText();
         asserterror G.GetUser('u'); s += '|' + GetLastErrorText();
         exit(s);",
        ": Text",
    );
    let t = text(r);
    let m = |name: &str| {
        format!(
            "A call to Microsoft.Dynamics.Nav.AzureADGraphClient.GraphQuery.{name} failed with this message: Object reference not set to an instance of an object."
        )
    };
    assert_eq!(
        t,
        format!(
            "NoNo[]|{}#DotNetInvoke:NullReference|{}|{}|{}",
            m("GetGroupPage"),
            m("GetGroupInfo"),
            m("FindFirstGroupInfoForDisplayName"),
            m("GetUser")
        )
    );
    assert_eq!(host.log(), vec![]);
}

// GetCurrentUser on a tier without Entra ID returns without an error and leaves the UserInfo null (probe agent,
// BC 29.0.54011.55816, OnPrem codeunit 61296, tests/al-language-onprem/session/PZAzureAdUser.al): Microsoft's path
// (`Azure AD Graph User`.InitializeGraphUser) then asks for the user's authentication object id. The runtime
// answers it itself; nothing goes to .NET.
#[test]
fn graph_get_current_user_returns_null_without_an_error() {
    let host = no_host();
    let r = run(
        &host,
        "G: DotNet GraphQuery; U: DotNet UserInfo; s: Text;",
        "G := G.GraphQuery(); U := G.GetCurrentUser(); exit(Format(IsNull(U)));",
        ": Text",
    );
    assert_eq!(text(r), "Yes");
    assert_eq!(host.log(), vec![]);
}

// GetTenantDetail answers a TenantInfo, not null, whose InitialDomain, CountryLetterCode and PreferredLanguage are
// empty: Microsoft's Azure AD Tenant Impl. raises "Failed to retrieve ..." for a null TenantInfo, and the corpus
// test expects '' from all three (session/TestEnvironmentUserTenantContracts.al:213-232).
#[test]
fn graph_get_tenant_detail_answers_an_empty_tenant_info() {
    let host = no_host();
    let r = run(
        &host,
        "G: DotNet GraphQuery; T: DotNet TenantInfo;",
        "G := G.GraphQuery(); T := G.GetTenantDetail(); if IsNull(T) then exit('null'); \
         exit('[' + T.InitialDomain() + '|' + T.CountryLetterCode() + '|' + T.PreferredLanguage() + ']');",
        ": Text",
    );
    assert_eq!(text(r), "[||]");
    assert_eq!(host.log(), vec![]);
}

// Microsoft's CDS Integration Impl.GetOptionSetMetadata: `Result.Add(Pair.Key, Pair.Value)` on a
// `Dictionary of [Integer, Text]` takes .NET results as the element types.
#[test]
fn a_dotnet_result_is_added_to_a_collection_as_its_element_type() {
    let host = Fake::new(|r| match r {
        Req::Invoke(_, m, _) if m == "Num" => reply(DotNetValue::Int32(3)),
        Req::Invoke(_, m, _) if m == "Name" => reply(DotNetValue::Str("Abc".into())),
        other => panic!("{other:?}"),
    });
    let r = run(
        &host,
        "C: DotNet Convert; D: Dictionary of [Integer, Text]; L: List of [Text];",
        "D.Add(C.Num(), C.Name()); L.Add(C.Name()); exit(D.Get(3) + L.Get(1));",
        ": Text",
    );
    assert_eq!(text(r), "AbcAbc");
}

/// `Permissions Mock`.Start/Stop/ClearAssignments (codeunit 131006) construct, clear and dispose the platform's
/// `PermissionTestHelper`. With no permission set assigned the user's own permissions apply (corpus
/// permissions/TestPermissionsMockEnforcement.al:74-97), and Start, Stop and ClearAssignments are accepted
/// (permissions/TestPermissionsMockLifecycle.al:29-82). BC's test runner starts and stops the mock around every
/// test method (Test Runner app, TestRunnerMgt.Codeunit.al:105 and :127).
#[test]
fn the_permission_test_helper_starts_clears_and_stops_without_the_host() {
    let host = Fake::new(|r| panic!("the host must not be called: {r:?}"));
    let r = run(
        &host,
        "H: DotNet PermissionTestHelper;",
        "H := H.PermissionTestHelper(); H.Clear(); H.Dispose(); exit('done');",
        ": Text",
    );
    assert_eq!(text(r), "done");
    assert_eq!(host.log(), vec![]);
}

/// Assigning a permission set lowers the session's permissions (enforced on BC: corpus
/// permissions/TestPermissionsMockEnforcement.al:28-47); permissions are not enforced here, so it is refused.
#[test]
fn adding_an_effective_permission_set_is_refused() {
    let host = Fake::new(|r| panic!("the host must not be called: {r:?}"));
    let r = run(
        &host,
        "H: DotNet PermissionTestHelper;",
        "H := H.PermissionTestHelper(); H.AddEffectivePermissionSet('D365 BASIC');",
        "",
    );
    let why = unsupported(&r);
    assert!(
        why.contains("AddEffectivePermissionSet") && why.contains("lowers"),
        "{why}"
    );
}

/// A .NET String passed straight to SecretStrSubstNo becomes the secret's content
/// (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al:463, QA_StringResult_IntoSecretText `SecretStrSubstNo`).
#[test]
fn a_dotnet_string_passed_to_secret_str_subst_no_is_its_content() {
    let host = convert_script();
    let r = run(
        &host,
        "C: DotNet Convert; S: SecretText;",
        "S := SecretStrSubstNo('Basic %1', C.ToBase64String('u:p')); exit(S.Unwrap());",
        ": Text",
    );
    assert_eq!(text(r), "Basic b64(u:p)");
}

/// Only a String is pinned: another .NET kind passed to SecretStrSubstNo is refused when it runs.
#[test]
fn a_dotnet_integer_passed_to_secret_str_subst_no_is_refused() {
    let host = Fake::new(|r| match r {
        Req::Invoke(DotNetTarget::Static(t), m, _) if t.name == "System.Convert" && m == "ToInt32" => {
            reply(DotNetValue::Int32(3))
        }
        other => panic!("unexpected request {other:?}"),
    });
    let r = run(
        &host,
        "C: DotNet Convert; S: SecretText;",
        "S := SecretStrSubstNo('n %1', C.ToInt32('3')); exit(S.Unwrap());",
        ": Text",
    );
    let why = unsupported(&r);
    assert!(why.contains("SecretText") || why.contains("not pinned"), "{why}");
}

/// A .NET String assigned to a SecretText is its content, and a null string an empty SecretText
/// (TestDotNetValueKindsContracts.al:463, QA_StringResult_IntoSecretText cases 1 and 4).
#[test]
fn a_dotnet_string_assigned_to_a_secret_text_is_its_content() {
    let host = convert_script();
    let r = run(
        &host,
        "C: DotNet Convert; S: SecretText;",
        "S := C.ToBase64String('u:p'); exit(Format(S.IsEmpty()) + S.Unwrap());",
        ": Text",
    );
    assert_eq!(text(r), "Nob64(u:p)");
    let host = Fake::new(|r| match r {
        Req::Invoke(DotNetTarget::Static(t), m, _) if t.name == "System.Convert" && m == "ToBase64String" => {
            reply(DotNetValue::NullString)
        }
        other => panic!("unexpected request {other:?}"),
    });
    let r = run(
        &host,
        "C: DotNet Convert; S: SecretText;",
        "S := C.ToBase64String('u:p'); exit(Format(S.IsEmpty()) + S.Unwrap());",
        ": Text",
    );
    assert_eq!(text(r), "Yes");
}

// ---- folders staged for the company build -----------------------------------------------

/// A file system that, like the company build's resource folder (al-testrun company_data.rs), lists one staged folder.
struct Staged;

impl al_host::FileSystem for Staged {
    fn read(&self, _: &str) -> Option<Vec<u8>> {
        None
    }
    fn write(&mut self, _: &str, _: &[u8]) -> Result<(), String> {
        Err("read only".into())
    }
    fn exists(&self, _: &str) -> bool {
        false
    }
    fn remove(&mut self, _: &str) -> bool {
        false
    }
    fn staged_folder_files(&self, dir: &str) -> Option<Vec<String>> {
        (dir == "\\TEMP\\..\\WalkMeTour").then(|| vec![format!("{dir}\\a.png"), format!("{dir}\\b.png")])
    }
}

const GET_FILES: &str = "Files := Directory.GetFiles(Path); exit(Format(Files.GetLength(0)));";

fn run_staged(host: &Fake, path: &str, files: Option<Staged>) -> Ran {
    let (program, blocked, oi, ri) = compile(
        "Directory: DotNet Directory; Files: DotNet Array; Path: Text;",
        &format!("Path := '{path}'; {GET_FILES}"),
        ": Text",
    );
    let engine = Engine::with_dotnet(program, blocked, cfg(host));
    let h = al_host::Host::default();
    let mut s = Session::with_host(
        engine,
        match files {
            Some(f) => h.with_files(f),
            None => h,
        },
    );
    let inst = s.instance(oi);
    let result = s.call(oi, ri, inst, vec![]).map(|(v, _)| v);
    Ran {
        result,
        session: Some(s),
    }
}

fn split_script() -> Fake {
    Fake::new(|r| match r {
        Req::Invoke(DotNetTarget::Static(t), m, a)
            if t.name == "System.Text.RegularExpressions.Regex" && m == "Split" =>
        {
            assert_eq!(
                a.iter().map(|a| a.value.clone()).collect::<Vec<_>>(),
                vec![
                    DotNetValue::Str("\\TEMP\\..\\WalkMeTour\\a.png\n\\TEMP\\..\\WalkMeTour\\b.png".into()),
                    DotNetValue::Str("\n".into())
                ]
            );
            reply(handle(7, "System.String[]"))
        }
        Req::Invoke(DotNetTarget::Instance(7), m, _) if m == "GetLength" => reply(DotNetValue::Int32(2)),
        other => panic!("unexpected request {other:?}"),
    })
}

#[test]
fn directory_get_files_of_a_folder_staged_for_the_company_build_is_a_string_array_of_its_files() {
    let host = split_script();
    assert_eq!(text(run_staged(&host, "\\TEMP\\..\\WalkMeTour", Some(Staged))), "2");
}

#[test]
fn directory_get_files_of_any_other_folder_stays_refused_and_never_reaches_the_host() {
    for (path, files) in [("\\TEMP\\..\\Other", Some(Staged)), ("\\TEMP\\..\\WalkMeTour", None)] {
        let host = split_script();
        let r = run_staged(&host, path, files);
        assert!(unsupported(&r).contains("System.IO.Directory"), "{}", unsupported(&r));
        assert!(host.calls().is_empty(), "{:?}", host.calls());
    }
}

/// A staged folder as the DemoTool's PostingExchangeDefinitions: XML files in mixed case, files the pattern leaves out,
/// and an empty one.
struct StagedDefs;

const DEFS: &str = "\\TEMP\\\\..\\PostingExchangeDefinitions";

impl al_host::FileSystem for StagedDefs {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        match path.rsplit('\\').next()? {
            "b.XML" => Some(b"<b/>".to_vec()),
            "c.xml" => Some(b"<cc/>".to_vec()),
            "Empty.xml" => Some(Vec::new()),
            _ => None,
        }
    }
    fn write(&mut self, _: &str, _: &[u8]) -> Result<(), String> {
        Err("read only".into())
    }
    fn exists(&self, _: &str) -> bool {
        false
    }
    fn remove(&mut self, _: &str) -> bool {
        false
    }
    fn staged_folder_files(&self, dir: &str) -> Option<Vec<String>> {
        (dir == DEFS).then(|| {
            ["b.XML", "c.xml", "c.xml.bak", "d.txt", "Empty.xml"]
                .iter()
                .map(|n| format!("{dir}/{n}"))
                .collect()
        })
    }
}

const ENUMERATE_XML: &str = "DirectoryInfo := DirectoryInfo.DirectoryInfo(Path);
    if not DirectoryInfo.Exists then exit('missing');
    Enumerator := DirectoryInfo.GetFiles(Pattern).GetEnumerator();
    while Enumerator.MoveNext() do begin
        FileInfo := Enumerator.Current();
        R += FileInfo.FullName + '=' + Format(FileInfo.Length) + ';';
    end;
    exit(R);";

fn run_staged_defs(path: &str, pattern: &str, files: bool) -> (Ran, Fake) {
    let host = Fake::new(|r| panic!("unexpected request {r:?}"));
    let (program, blocked, oi, ri) = compile(
        "DirectoryInfo: DotNet DirectoryInfo; FileInfo: DotNet FileInfo; Enumerator: DotNet IEnumerator; Path: Text; Pattern: Text; R: Text;",
        &format!("Path := '{path}'; Pattern := '{pattern}'; {ENUMERATE_XML}"),
        ": Text",
    );
    let engine = Engine::with_dotnet(program, blocked, cfg(&host));
    let h = al_host::Host::default();
    let mut s = Session::with_host(engine, if files { h.with_files(StagedDefs) } else { h });
    let inst = s.instance(oi);
    let result = s.call(oi, ri, inst, vec![]).map(|(v, _)| v);
    (
        Ran {
            result,
            session: Some(s),
        },
        host,
    )
}

#[test]
fn directory_info_of_a_folder_staged_for_the_company_build_lists_its_xml_files_in_windows_order() {
    // Windows BC: '*.xml' selects a.xml, b.XML, c.xml without case and leaves out e.xml.bak and d.txt, in name order
    // (windows-check-results 2026-10-22 QX 14, corpus QxDotNetDemoShapes.al:73); FullName joins with a backslash.
    let (r, host) = run_staged_defs(DEFS, "*.xml", true);
    assert_eq!(text(r), format!("{DEFS}\\b.XML=4;{DEFS}\\c.xml=5;{DEFS}\\Empty.xml=0;"));
    assert!(host.calls().is_empty(), "{:?}", host.calls());
}

#[test]
fn directory_info_of_any_other_folder_or_pattern_stays_refused_and_never_reaches_the_host() {
    for (path, pattern, files) in [
        ("\\TEMP\\..\\Other", "*.xml", true),
        (DEFS, "*.xml", false),
        (DEFS, "*.txt", true),
        (DEFS, "*", true),
    ] {
        let (r, host) = run_staged_defs(path, pattern, files);
        assert!(
            unsupported(&r).contains("System.IO.DirectoryInfo"),
            "{}",
            unsupported(&r)
        );
        assert!(host.calls().is_empty(), "{:?}", host.calls());
    }
}
