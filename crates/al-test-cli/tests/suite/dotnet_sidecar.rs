//! The .NET sidecar and its host (ADR-014), against the real `tools/dotnet-sidecar`. These need a
//! .NET 8 runtime and SDK. Without them a test skips with a visible message; with
//! `ALRT_REQUIRE_DOTNET=1` (CI) a missing .NET fails instead, so a skip never reads as a pass.

use al_host::dotnet::*;
use al_test_cli::dotnet::{SidecarConfig, SidecarHost};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn config() -> SidecarConfig {
    SidecarConfig {
        source_root: Some(root()),
        cache_dir: Some(Path::new(env!("CARGO_TARGET_TMPDIR")).join("dotnet-sidecar-cache")),
        ..Default::default()
    }
}

/// A host on the real sidecar, or `None` (skip) when this machine has no usable .NET.
fn host_with(cfg: SidecarConfig) -> Option<Arc<SidecarHost>> {
    let h = Arc::new(SidecarHost::new(cfg));
    match h.availability() {
        Ok(()) => Some(h),
        Err(why) => {
            assert!(
                std::env::var("ALRT_REQUIRE_DOTNET").is_err(),
                "ALRT_REQUIRE_DOTNET is set but .NET is unavailable: {why}"
            );
            eprintln!("SKIPPED (no usable .NET): DotNet {why}");
            None
        }
    }
}

fn host() -> Option<Arc<SidecarHost>> {
    host_with(config())
}

fn ty(name: &str) -> DotNetType {
    DotNetType {
        assembly: "netstandard".into(),
        culture: Some("neutral".into()),
        public_key_token: Some("cc7b13ffcd2ddd51".into()),
        name: name.into(),
        ..Default::default()
    }
}

fn st(name: &str) -> DotNetTarget {
    DotNetTarget::Static(ty(name))
}

fn arg(v: DotNetValue) -> DotNetArg {
    DotNetArg {
        value: v,
        by_ref: false,
    }
}

fn s(v: &str) -> DotNetArg {
    arg(DotNetValue::Str(v.into()))
}

fn handle_of(r: &DotNetReply) -> u64 {
    match &r.value {
        DotNetValue::Handle { handle, .. } => *handle,
        other => panic!("expected a handle, got {other:?}"),
    }
}

fn text_of(r: &DotNetReply) -> &str {
    match &r.value {
        DotNetValue::Str(s) => s,
        other => panic!("expected text, got {other:?}"),
    }
}

fn exception(r: Result<DotNetReply, DotNetFailure>) -> DotNetException {
    match r {
        Err(DotNetFailure::Exception(e)) => e,
        other => panic!("expected a .NET exception, got {other:?}"),
    }
}

fn refused(r: Result<DotNetReply, DotNetFailure>) -> String {
    match r {
        Err(DotNetFailure::Refused(s)) => s,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// ---- the spike's checks ------------------------------------------------------------------

#[test]
fn convert_base64_round_trip_through_real_framework_types() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let utf8 = sess.get_property(st("System.Text.Encoding"), "UTF8").unwrap();
    let enc = handle_of(&utf8);
    let bytes = sess
        .invoke(DotNetTarget::Instance(enc), "GetBytes", vec![s("hello")])
        .unwrap();
    assert!(
        matches!(&bytes.value, DotNetValue::Handle { ty, .. } if ty == "System.Byte[]"),
        "arrays are handles"
    );
    let b = handle_of(&bytes);
    let r = sess
        .invoke(
            st("System.Convert"),
            "ToBase64String",
            vec![arg(DotNetValue::Handle {
                handle: b,
                ty: "System.Byte[]".into(),
            })],
        )
        .unwrap();
    assert_eq!(text_of(&r), "aGVsbG8=");
    // The overload with an enum argument: 12 bytes with line breaks every 76 characters is the same text.
    let r = sess
        .invoke(
            st("System.Convert"),
            "ToBase64String",
            vec![
                arg(DotNetValue::Handle {
                    handle: b,
                    ty: "System.Byte[]".into(),
                }),
                arg(DotNetValue::Enum {
                    ty: "System.Base64FormattingOptions".into(),
                    name: "InsertLineBreaks".into(),
                    ordinal: 1,
                }),
            ],
        )
        .unwrap();
    assert_eq!(text_of(&r), "aGVsbG8=");
    // And an integer for the enum.
    let r = sess
        .invoke(
            st("System.Convert"),
            "ToBase64String",
            vec![
                arg(DotNetValue::Handle {
                    handle: b,
                    ty: "System.Byte[]".into(),
                }),
                arg(DotNetValue::Int32(0)),
            ],
        )
        .unwrap();
    assert_eq!(text_of(&r), "aGVsbG8=");
    let back = sess
        .invoke(st("System.Convert"), "FromBase64String", vec![s("aGVsbG8=")])
        .unwrap();
    let r = sess
        .get_property(DotNetTarget::Instance(handle_of(&back)), "Length")
        .unwrap();
    assert_eq!(r.value, DotNetValue::Int32(5));
    sess.close();
}

#[test]
fn a_memory_stream_takes_writes_reports_its_length_and_releasing_its_handle_frees_it() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let ms = handle_of(&sess.construct(&ty("System.IO.MemoryStream"), vec![]).unwrap());
    let w = sess
        .invoke(
            DotNetTarget::Instance(ms),
            "WriteByte",
            vec![arg(DotNetValue::Byte(65))],
        )
        .unwrap();
    assert_eq!(w.value, DotNetValue::Null, "a void method returns null");
    let len = sess.get_property(DotNetTarget::Instance(ms), "Length").unwrap();
    assert_eq!(len.value, DotNetValue::Int64(1));
    sess.set_property(DotNetTarget::Instance(ms), "Position", DotNetValue::Int32(0))
        .unwrap();
    let b = sess.invoke(DotNetTarget::Instance(ms), "ReadByte", vec![]).unwrap();
    assert_eq!(b.value, DotNetValue::Int32(65));
    sess.release(&[ms]);
    let why = refused(sess.get_property(DotNetTarget::Instance(ms), "Length"));
    assert!(why.contains("unknown object handle"), "{why}");
    sess.close();
}

#[test]
fn exceptions_carry_the_clr_type_and_message_and_the_sidecar_survives_them() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let e = exception(sess.invoke(st("System.Convert"), "FromBase64String", vec![s("not base64!")]));
    assert_eq!(e.clr_type, "System.FormatException");
    assert!(e.message.contains("Base-64"), "{}", e.message);
    // An exception with an inner exception: the type initialiser of a faulty type is not at hand,
    // so use a constructor that wraps: new Uri with a bad string has no inner. Parse int overflow:
    let e = exception(sess.invoke(st("System.Convert"), "ToInt32", vec![s("99999999999")]));
    assert_eq!(e.clr_type, "System.OverflowException");
    let ok = sess.invoke(st("System.Convert"), "ToInt32", vec![s("12")]).unwrap();
    assert_eq!(ok.value, DotNetValue::Int32(12), "alive after exceptions");
    sess.close();
}

#[test]
fn a_missing_type_or_member_is_a_refusal_not_an_exception() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let why = refused(sess.invoke(st("System.NoSuchType"), "X", vec![]));
    assert!(why.contains("type 'System.NoSuchType' not found"), "{why}");
    let why = refused(sess.invoke(st("System.Convert"), "NoSuchMember", vec![]));
    assert!(
        why.contains("member 'NoSuchMember' not found on 'System.Convert'"),
        "{why}"
    );
    let why = refused(sess.invoke(st("System.Convert"), "ToInt32", vec![]));
    assert!(why.contains("no overload taking 0 argument"), "{why}");
    let mut bad = ty("System.Convert");
    bad.assembly = "No.Such.Assembly".into();
    let why = refused(sess.invoke(DotNetTarget::Static(bad), "ToInt32", vec![s("1")]));
    assert!(why.contains("assembly 'No.Such.Assembly' could not be loaded"), "{why}");
    let why = refused(sess.invoke(st("System.Text.Encoding"), "GetBytes", vec![s("x")]));
    assert!(why.contains("instance member"), "{why}");
    sess.close();
}

// A method that is generic only, with its one type parameter as its return type (OpenXml's `T GetWorksheetByName<T>(string)`),
// is refused by a plain call with a text that says so, and runs when the call names the type argument.
#[test]
fn a_generic_only_method_is_refused_plainly_and_runs_closed_over_a_type_argument() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let sb = handle_of(
        &sess
            .construct(&ty("System.Text.StringBuilder"), vec![s("abc")])
            .unwrap(),
    );
    let mut unsafe_ty = ty("System.Runtime.CompilerServices.Unsafe");
    unsafe_ty.assembly = "System.Runtime".into();
    unsafe_ty.public_key_token = Some("b03f5f7f11d50a3a".into());
    let obj = arg(DotNetValue::Handle {
        handle: sb,
        ty: "System.Text.StringBuilder".into(),
    });
    let why = refused(sess.invoke(DotNetTarget::Static(unsafe_ty.clone()), "As", vec![obj.clone()]));
    assert!(why.contains("generic only"), "{why}");
    let r = sess
        .invoke_generic(
            DotNetTarget::Static(unsafe_ty),
            "As",
            &ty("System.Text.StringBuilder"),
            vec![obj],
        )
        .unwrap();
    let h2 = handle_of(&r);
    assert_eq!(
        text_of(&sess.invoke(DotNetTarget::Instance(h2), "ToString", vec![]).unwrap()),
        "abc"
    );
    sess.close();
}

// ---- ADR-014 conformance: sessions, restricted environment, deadlines ------------------

#[test]
fn static_state_does_not_leak_between_sessions() {
    let Some(h) = host() else { return };
    let mut a = h.open_session(&DotNetSessionConfig::default());
    a.invoke(st("System.AppContext"), "SetData", vec![s("k"), s("v")])
        .unwrap();
    assert_eq!(
        a.invoke(st("System.AppContext"), "GetData", vec![s("k")])
            .unwrap()
            .value,
        DotNetValue::Str("v".into())
    );
    let mut b = h.open_session(&DotNetSessionConfig::default());
    assert_eq!(
        b.invoke(st("System.AppContext"), "GetData", vec![s("k")])
            .unwrap()
            .value,
        DotNetValue::Null,
        "a new session is a new process"
    );
    a.close();
    b.close();
}

#[test]
fn the_sidecar_runs_with_en_us_culture_utc_a_cleared_environment_and_an_empty_working_directory() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let culture = sess
        .get_property(st("System.Globalization.CultureInfo"), "CurrentCulture")
        .unwrap();
    let name = sess
        .get_property(DotNetTarget::Instance(handle_of(&culture)), "Name")
        .unwrap();
    assert_eq!(name.value, DotNetValue::Str("en-US".into()));
    let tz = sess.get_property(st("System.TimeZoneInfo"), "Local").unwrap();
    let id = sess.get_property(DotNetTarget::Instance(handle_of(&tz)), "Id").unwrap();
    assert_eq!(id.value, DotNetValue::Str("UTC".into()));
    let user = sess
        .invoke(st("System.Environment"), "GetEnvironmentVariable", vec![s("USER")])
        .unwrap();
    assert_eq!(
        user.value,
        DotNetValue::NullString,
        "the parent's environment is not inherited"
    );
    let path = sess
        .invoke(st("System.Environment"), "GetEnvironmentVariable", vec![s("PATH")])
        .unwrap();
    assert_eq!(path.value, DotNetValue::NullString);
    let cwd = sess.get_property(st("System.Environment"), "CurrentDirectory").unwrap();
    let DotNetValue::Str(cwd) = cwd.value else { panic!() };
    assert!(cwd.contains("al-runtime-dotnet-") && cwd.ends_with("cwd"), "{cwd}");
    assert_eq!(std::fs::read_dir(&cwd).unwrap().count(), 0, "an empty temporary folder");
    sess.close();
    assert!(!Path::new(&cwd).exists(), "removed when the session closes");
}

#[test]
fn a_call_past_its_deadline_kills_the_process_and_every_later_call_is_refused() {
    let mut cfg = config();
    cfg.timeout = Duration::from_millis(1500);
    let Some(h) = host_with(cfg) else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    sess.invoke(st("System.Convert"), "ToInt32", vec![s("1")]).unwrap();
    let why = refused(sess.invoke(
        st("System.Threading.Thread"),
        "Sleep",
        vec![arg(DotNetValue::Int32(30_000))],
    ));
    assert!(why.contains("timed out") && why.contains("killed"), "{why}");
    let again = refused(sess.invoke(st("System.Convert"), "ToInt32", vec![s("1")]));
    assert_eq!(again, why, "the session stays dead");
    // The next session starts a new process.
    let mut next = h.open_session(&DotNetSessionConfig::default());
    assert!(next.invoke(st("System.Convert"), "ToInt32", vec![s("1")]).is_ok());
    next.close();
    sess.close();
}

#[cfg(unix)]
#[test]
fn no_process_starts_until_the_first_call() {
    // A stand-in `dotnet` that leaves a marker whenever it is run as the sidecar.
    let dir = tempfile::tempdir().unwrap();
    let dll = dir.path().join("AlDotNetSidecar.dll");
    std::fs::write(&dll, b"").unwrap();
    let marker = dir.path().join("started");
    let script = format!(
        "if [ \"$1\" = \"--list-runtimes\" ]; then echo 'Microsoft.NETCore.App 8.0.1 [/x]'; exit 0; fi\ntouch '{}'\nexit 1",
        marker.display()
    );
    let dotnet = fake_dotnet(dir.path(), &script);
    let h = SidecarHost::new(SidecarConfig {
        dotnet: Some(dotnet),
        sidecar: Some(dll),
        ..Default::default()
    });
    h.availability().unwrap();
    let sess = h.open_session(&DotNetSessionConfig::default());
    sess.close();
    assert!(!marker.exists(), "an unused session starts nothing");
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let _ = sess.invoke(st("System.Convert"), "ToInt32", vec![s("1")]);
    assert!(marker.exists(), "the first call starts the process");
    sess.close();
}

#[test]
fn assemblies_come_from_the_assembly_dir_by_simple_name() {
    // The sidecar itself is an assembly outside the framework: serve it by simple name from its folder.
    let Some(h0) = host() else { return };
    let dll = h0.sidecar_path().expect("available, so found");
    let mut cfg = config();
    cfg.assembly_dirs = vec![dll.parent().unwrap().to_path_buf()];
    let Some(h) = host_with(cfg) else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let mut t = ty("AlDotNetSidecar.Program");
    t.assembly = "AlDotNetSidecar".into();
    t.culture = None;
    t.public_key_token = None;
    let r = sess.get_property(DotNetTarget::Static(t), "ProtocolVersion");
    assert_eq!(r.unwrap().value, DotNetValue::Int32(4));
    sess.close();
}

// ---- ADR-024: values typed at run time ---------------------------------------------------

#[test]
fn a_null_string_result_says_it_is_a_string_and_a_null_object_does_not() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    // An unset environment variable: a method returning string.
    let r = sess
        .invoke(
            st("System.Environment"),
            "GetEnvironmentVariable",
            vec![s("ARRS_NO_SUCH_VARIABLE_61400")],
        )
        .unwrap();
    assert_eq!(r.value, DotNetValue::NullString);
    // A null object: `Type.GetType` of an unknown name returns a null Type.
    let r = sess
        .invoke(st("System.Type"), "GetType", vec![s("No.Such.Type")])
        .unwrap();
    assert_eq!(r.value, DotNetValue::Null);
    sess.close();
}

#[test]
fn a_date_time_offset_is_built_from_ticks_and_a_zero_offset() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    // 2024-01-15T12:00:00Z in ticks since 0001-01-01.
    let ticks: i64 = 638_408_208_000_000_000;
    let r = sess
        .construct(
            &ty("System.DateTimeOffset"),
            vec![arg(DotNetValue::Int64(ticks)), arg(DotNetValue::TimeSpan(0))],
        )
        .unwrap();
    let at = handle_of(&r);
    let local = sess.get_property(DotNetTarget::Instance(at), "LocalDateTime").unwrap();
    assert_eq!(local.value, DotNetValue::DateTime(ticks));
    let offset = sess.get_property(DotNetTarget::Instance(at), "Offset").unwrap();
    assert_eq!(offset.value, DotNetValue::TimeSpan(0));
    sess.close();
}

// ---- streams -------------------------------------------------------------------------------

#[test]
fn an_outstream_argument_becomes_a_bridge_and_what_dotnet_writes_comes_back_after_the_flush() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let w = sess
        .construct(&ty("System.IO.StreamWriter"), vec![arg(DotNetValue::OutStream)])
        .unwrap();
    assert_eq!(w.streams.bridges.len(), 1);
    let (idx, bridge) = w.streams.bridges[0];
    assert_eq!(idx, 0);
    let writer = handle_of(&w);
    let r = sess
        .invoke(DotNetTarget::Instance(writer), "Write", vec![s("hello")])
        .unwrap();
    assert!(r.streams.written.is_empty(), "buffered until the flush");
    let r = sess.invoke(DotNetTarget::Instance(writer), "Flush", vec![]).unwrap();
    assert_eq!(r.streams.written, vec![(bridge, b"hello".to_vec())]);
    let r = sess
        .invoke(DotNetTarget::Instance(writer), "Write", vec![s("!")])
        .unwrap();
    let r2 = sess.invoke(DotNetTarget::Instance(writer), "Flush", vec![]).unwrap();
    assert!(r.streams.written.is_empty());
    assert_eq!(r2.streams.written, vec![(bridge, b"!".to_vec())], "only the new bytes");
    sess.close();
}

// al-runner issue 6317: an OutStream over bytes its Blob holds hands .NET a stream that starts with them, at their end.
#[test]
fn a_seeded_outstream_bridge_holds_the_bytes_appends_after_them_and_refuses_a_write_over_them() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let w = sess
        .construct(
            &ty("System.IO.StreamWriter"),
            vec![arg(DotNetValue::OutStreamWith(b"abc".to_vec()))],
        )
        .unwrap();
    let (_, bridge) = w.streams.bridges[0];
    let writer = handle_of(&w);
    let base = handle_of(&sess.get_property(DotNetTarget::Instance(writer), "BaseStream").unwrap());
    let len = sess.get_property(DotNetTarget::Instance(base), "Length").unwrap();
    assert_eq!(len.value, DotNetValue::Int64(3), "the bridge holds the bytes");
    let pos = sess.get_property(DotNetTarget::Instance(base), "Position").unwrap();
    assert_eq!(pos.value, DotNetValue::Int64(3), "and stands at their end");
    sess.invoke(DotNetTarget::Instance(writer), "Write", vec![s("d")])
        .unwrap();
    let r = sess.invoke(DotNetTarget::Instance(writer), "Flush", vec![]).unwrap();
    assert_eq!(
        r.streams.written,
        vec![(bridge, b"d".to_vec())],
        "only what follows the seed comes back"
    );
    sess.set_property(DotNetTarget::Instance(base), "Position", DotNetValue::Int64(0))
        .unwrap();
    let why = refused(sess.invoke(
        DotNetTarget::Instance(base),
        "WriteByte",
        vec![arg(DotNetValue::Byte(1))],
    ));
    assert!(why.contains("over the bytes an AL OutStream held"), "{why}");
    sess.close();
}

#[test]
fn an_instream_argument_is_read_from_the_unread_bytes_and_the_consumed_count_comes_back() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let doc = handle_of(&sess.construct(&ty("System.Xml.XmlDocument"), vec![]).unwrap());
    let xml = b"<a><b>text</b></a>".to_vec();
    let r = sess
        .invoke(
            DotNetTarget::Instance(doc),
            "Load",
            vec![arg(DotNetValue::InStream(xml.clone()))],
        )
        .unwrap();
    assert_eq!(r.streams.consumed, vec![(0, xml.len())]);
    let root = sess
        .get_property(DotNetTarget::Instance(doc), "DocumentElement")
        .unwrap();
    let name = sess
        .get_property(DotNetTarget::Instance(handle_of(&root)), "Name")
        .unwrap();
    assert_eq!(name.value, DotNetValue::Str("a".into()));
    sess.close();
}

#[test]
fn by_reference_parameters_come_back_by_position() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    // Int32.TryParse(string, out int): the AL variable is passed by reference.
    let r = sess
        .invoke(
            st("System.Int32"),
            "TryParse",
            vec![
                s("42"),
                DotNetArg {
                    value: DotNetValue::Int32(0),
                    by_ref: true,
                },
            ],
        )
        .unwrap();
    assert_eq!(r.value, DotNetValue::Bool(true));
    assert_eq!(r.by_ref, vec![(1, DotNetValue::Int32(42))]);
    sess.close();
}

#[test]
fn values_round_trip_by_value_and_enums_keep_their_clr_type() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let day = 638_396_640_000_000_000_i64;
    let r = sess
        .invoke(st("System.DateTime"), "Parse", vec![s("2024-01-01T12:00:00")])
        .unwrap();
    assert_eq!(r.value, DotNetValue::DateTime(day + 432_000_000_000));
    let r = sess.invoke(
        st("System.TimeSpan"),
        "FromMilliseconds",
        vec![arg(DotNetValue::Decimal("1500".into()))],
    );
    // Decimal has no TimeSpan.FromMilliseconds overload (it takes double): a numeric conversion applies.
    assert_eq!(r.unwrap().value, DotNetValue::TimeSpan(15_000_000));
    let r = sess
        .invoke(
            st("System.Guid"),
            "Parse",
            vec![s("01234567-89ab-cdef-0123-456789abcdef")],
        )
        .unwrap();
    assert_eq!(
        r.value,
        DotNetValue::Guid([
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF
        ])
    );
    let r = sess.invoke(st("System.Decimal"), "Parse", vec![s("2.50")]).unwrap();
    assert_eq!(r.value, DotNetValue::Decimal("2.50".into()));
    let r = sess
        .get_property(st("System.Base64FormattingOptions"), "InsertLineBreaks")
        .unwrap();
    assert_eq!(
        r.value,
        DotNetValue::Enum {
            ty: "System.Base64FormattingOptions".into(),
            name: "InsertLineBreaks".into(),
            ordinal: 1
        }
    );
    sess.close();
}

#[test]
fn generic_type_names_keep_the_clr_spelling() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let list = sess
        .construct(
            &ty("System.Collections.Generic.List`1[[System.String, mscorlib]]"),
            vec![],
        )
        .unwrap();
    let l = handle_of(&list);
    sess.invoke(DotNetTarget::Instance(l), "Add", vec![s("x")]).unwrap();
    let n = sess.get_property(DotNetTarget::Instance(l), "Count").unwrap();
    assert_eq!(n.value, DotNetValue::Int32(1));
    sess.close();
}

// ---- unavailability (ADR-014 section 11) --------------------------------------------------

#[cfg(unix)]
fn fake_dotnet(dir: &Path, script: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let p = dir.join("dotnet");
    std::fs::write(&p, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    // Another test thread may have forked while the script was open for writing, which makes an
    // exec fail with ETXTBSY (26) for a moment. Run it once, retrying, so the test sees a stable file.
    for _ in 0..100 {
        match std::process::Command::new(&p).arg("--list-runtimes").output() {
            Err(e) if e.raw_os_error() == Some(26) => std::thread::sleep(Duration::from_millis(20)),
            _ => break,
        }
    }
    p
}

#[test]
fn a_missing_dotnet_is_unavailable_with_the_lookup_places_named() {
    let h = SidecarHost::new(SidecarConfig {
        dotnet: Some(PathBuf::from("/nonexistent/dotnet")),
        ..Default::default()
    });
    let e = h.availability().unwrap_err();
    assert!(e.0.starts_with("needs the .NET 8 runtime; 'dotnet' not found"), "{e}");
}

#[cfg(unix)]
#[test]
fn a_runtime_of_the_wrong_major_is_unavailable_and_names_what_was_found() {
    let dir = tempfile::tempdir().unwrap();
    let dotnet = fake_dotnet(dir.path(), "echo 'Microsoft.NETCore.App 9.0.15 [/x]'");
    let h = SidecarHost::new(SidecarConfig {
        dotnet: Some(dotnet.clone()),
        ..Default::default()
    });
    let e = h.availability().unwrap_err();
    assert!(
        e.0.contains("found 9.0.15") && e.0.contains("needs Microsoft.NETCore.App 8.x"),
        "{e}"
    );
    // --dotnet-allow-roll-forward gets past this check (and then needs a sidecar build).
    let h = SidecarHost::new(SidecarConfig {
        dotnet: Some(dotnet),
        allow_roll_forward: true,
        sidecar: Some(dir.path().join("missing.dll")),
        ..Default::default()
    });
    let e = h.availability().unwrap_err();
    assert!(e.0.contains("sidecar not built"), "{e}");
}

#[cfg(unix)]
#[test]
fn an_unbuilt_sidecar_is_unavailable_with_the_publish_command() {
    let dir = tempfile::tempdir().unwrap();
    let dotnet = fake_dotnet(dir.path(), "echo 'Microsoft.NETCore.App 8.0.1 [/x]'");
    let h = SidecarHost::new(SidecarConfig {
        dotnet: Some(dotnet),
        sidecar: Some(dir.path().join("AlDotNetSidecar.dll")),
        ..Default::default()
    });
    let e = h.availability().unwrap_err();
    assert!(e.0.contains("sidecar not built"), "{e}");
}

#[cfg(unix)]
#[test]
fn a_protocol_mismatch_is_a_refusal_naming_both_versions() {
    let dir = tempfile::tempdir().unwrap();
    let dll = dir.path().join("AlDotNetSidecar.dll");
    std::fs::write(&dll, b"").unwrap();
    // `dotnet --list-runtimes` answers; run as `dotnet <dll>` it writes one hello frame of version 99.
    let reply = r#"{"id":1,"ok":true,"value":{"protocol":99,"compatible":false,"major":8}}"#;
    let len = reply.len();
    let script = [
        "if [ \"$1\" = \"--list-runtimes\" ]; then echo 'Microsoft.NETCore.App 8.0.1 [/x]'; exit 0; fi".to_string(),
        format!("printf '\\{len:03o}\\000\\000\\000'"),
        format!("printf '%s' '{reply}'"),
        "sleep 5".to_string(),
    ]
    .join("\n");
    let dotnet = fake_dotnet(dir.path(), &script);
    let h = SidecarHost::new(SidecarConfig {
        dotnet: Some(dotnet),
        sidecar: Some(dll),
        ..Default::default()
    });
    h.availability().expect("looks fine until it starts");
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let why = refused(sess.invoke(st("System.Convert"), "ToInt32", vec![s("1")]));
    assert!(
        why.contains("protocol mismatch") && why.contains("99") && why.contains('4'),
        "{why}"
    );
    sess.close();
}

#[cfg(unix)]
#[test]
fn a_sidecar_that_dies_is_reported_with_its_exit_status_and_last_stderr_line() {
    let dir = tempfile::tempdir().unwrap();
    let dll = dir.path().join("AlDotNetSidecar.dll");
    std::fs::write(&dll, b"").unwrap();
    let script = "if [ \"$1\" = \"--list-runtimes\" ]; then echo 'Microsoft.NETCore.App 8.0.1 [/x]'; exit 0; fi\n\
                  echo 'boom: out of memory' >&2\nexit 3";
    let dotnet = fake_dotnet(dir.path(), script);
    let h = SidecarHost::new(SidecarConfig {
        dotnet: Some(dotnet),
        sidecar: Some(dll),
        ..Default::default()
    });
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let why = refused(sess.invoke(st("System.Convert"), "ToInt32", vec![s("1")]));
    assert!(
        why.contains("exit") && why.contains("3") && why.contains("boom: out of memory"),
        "{why}"
    );
    sess.close();
}

#[test]
fn the_marshalling_table_against_the_real_sidecar_decodes_each_row_to_its_clr_type() {
    // The same rows as the fake-host table in al-vm/tests/dotnet.rs, now read back by .NET itself:
    // Convert.GetTypeCode names the CLR type the sidecar decoded a wire value to.
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let rows: Vec<(DotNetValue, &str)> = vec![
        (DotNetValue::Str("x".into()), "String"),
        (DotNetValue::Int32(-5), "Int32"),
        (DotNetValue::Int64(5_000_000_000), "Int64"),
        (DotNetValue::Byte(200), "Byte"),
        (DotNetValue::Char(65), "Char"),
        (DotNetValue::Bool(true), "Boolean"),
        (DotNetValue::Decimal("1.5".into()), "Decimal"),
        (DotNetValue::DateTime(638_396_640_000_000_000), "DateTime"),
        (DotNetValue::Null, "Empty"),
    ];
    for (wire, want) in rows {
        let r = sess
            .invoke(st("System.Convert"), "GetTypeCode", vec![arg(wire.clone())])
            .unwrap_or_else(|e| panic!("{wire:?}: {e:?}"));
        let DotNetValue::Enum { name, .. } = r.value else {
            panic!("{wire:?}")
        };
        assert_eq!(name, want, "{wire:?}");
    }
    // Guid and TimeSpan have no TypeCode: their text shows the decoded value.
    let g = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
    ];
    let r = sess.invoke(
        st("System.Convert"),
        "ToString",
        vec![arg(DotNetValue::Guid(g)), arg(DotNetValue::Null)],
    );
    let _ = r; // overload with an IFormatProvider; the one-argument form is below
    let r = sess
        .invoke(
            st("System.String"),
            "Concat",
            vec![arg(DotNetValue::Guid(g)), arg(DotNetValue::Str("!".into()))],
        )
        .unwrap();
    assert_eq!(text_of(&r), "01234567-89ab-cdef-0123-456789abcdef!");
    let r = sess
        .invoke(
            st("System.String"),
            "Concat",
            vec![
                arg(DotNetValue::TimeSpan(15_000_000)),
                arg(DotNetValue::Str("!".into())),
            ],
        )
        .unwrap();
    assert_eq!(text_of(&r), "00:00:01.5000000!");
    // An AL Integer given to a .NET enum parameter becomes the enum.
    let bytes = handle_of(
        &sess
            .invoke(st("System.Convert"), "FromBase64String", vec![s("aGk=")])
            .unwrap(),
    );
    let r = sess
        .invoke(
            st("System.Convert"),
            "ToBase64String",
            vec![
                arg(DotNetValue::Handle {
                    handle: bytes,
                    ty: "System.Byte[]".into(),
                }),
                arg(DotNetValue::Int32(0)),
            ],
        )
        .unwrap();
    assert_eq!(text_of(&r), "aGk=");
    sess.close();
}

// ---- start-up is measured apart from the calls (perf budget, sidecar-start) -------------

#[test]
fn the_host_reports_how_many_sidecars_started_and_how_long_starting_took() {
    let Some(h) = host() else { return };
    let none = h.stats();
    assert_eq!(none.started, 0, "nothing started yet: {none:?}");
    // Finding `dotnet` (it runs `dotnet --list-runtimes`) is timed apart from the processes.
    assert!(none.find_total > Duration::ZERO, "{none:?}");
    let mut a = h.open_session(&DotNetSessionConfig::default());
    a.invoke(st("System.Convert"), "ToInt32", vec![s("1")]).unwrap();
    a.invoke(st("System.Convert"), "ToInt32", vec![s("2")]).unwrap();
    let one = h.stats();
    assert_eq!(one.started, 1, "two calls in one session are one process: {one:?}");
    assert!(
        one.start_total > Duration::ZERO && one.start_max == one.start_total,
        "{one:?}"
    );
    assert_eq!(one.calls, 2, "{one:?}");
    let mut b = h.open_session(&DotNetSessionConfig::default());
    b.invoke(st("System.Convert"), "ToInt32", vec![s("3")]).unwrap();
    let two = h.stats();
    assert_eq!(two.started, 2, "{two:?}");
    assert!(
        two.start_total >= two.start_max && two.start_max > Duration::ZERO,
        "{two:?}"
    );
    a.close();
    b.close();
}

// ---- standby sidecars: started in the background, used once (ADR-014 section 6) --------

/// Waits until `n` sidecar processes have finished starting.
fn wait_for_started(h: &SidecarHost, n: usize) {
    let until = std::time::Instant::now() + Duration::from_secs(60);
    while h.stats().started < n {
        assert!(
            std::time::Instant::now() < until,
            "no {n} standby started: {:?}",
            h.stats()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn entries_of(dir: &Path) -> usize {
    std::fs::read_dir(dir).unwrap().count()
}

#[test]
fn a_session_takes_a_standby_that_started_in_the_background_and_waits_for_nothing() {
    let Some(h) = host() else { return };
    h.prewarm(1);
    wait_for_started(&h, 1);
    let ready = h.stats();
    let mut a = h.open_session(&DotNetSessionConfig::default());
    a.invoke(st("System.Convert"), "ToInt32", vec![s("1")]).unwrap();
    let used = h.stats();
    assert_eq!(used.from_standby, 1, "{used:?}");
    // Starting took real time; the session did not wait for it.
    assert!(ready.start_total > Duration::ZERO, "{ready:?}");
    // (Finding `dotnet`, and building the sidecar on a cold cache, was waited for before.)
    let waited = used.waited_total - ready.waited_total;
    assert!(
        waited < ready.start_total / 2,
        "waited {waited:?}, start {:?}",
        ready.start_total
    );
    a.close();
    h.shutdown();
}

#[test]
fn a_session_that_calls_while_the_standby_is_starting_waits_for_it_instead_of_starting_another() {
    let Some(h) = host() else { return };
    h.prewarm(1);
    let mut a = h.open_session(&DotNetSessionConfig::default());
    a.invoke(st("System.Convert"), "ToInt32", vec![s("1")]).unwrap();
    assert_eq!(h.stats().from_standby, 1, "{:?}", h.stats());
    a.close();
    h.shutdown();
}

#[test]
fn a_standby_is_used_by_one_session_only_so_static_state_still_does_not_leak() {
    let Some(h) = host() else { return };
    h.prewarm(2);
    wait_for_started(&h, 2);
    let mut a = h.open_session(&DotNetSessionConfig::default());
    a.invoke(st("System.AppContext"), "SetData", vec![s("k"), s("v")])
        .unwrap();
    let mut b = h.open_session(&DotNetSessionConfig::default());
    assert_eq!(
        b.invoke(st("System.AppContext"), "GetData", vec![s("k")])
            .unwrap()
            .value,
        DotNetValue::Null,
        "a second session gets its own process, standby or not"
    );
    a.close();
    b.close();
    h.shutdown();
}

#[test]
fn a_taken_standby_is_replaced_and_shutdown_removes_the_ones_nobody_used() {
    let work = tempfile::tempdir().unwrap();
    let Some(h) = host_with(SidecarConfig {
        work_root: Some(work.path().to_path_buf()),
        ..config()
    }) else {
        return;
    };
    h.prewarm(1);
    wait_for_started(&h, 1);
    let mut a = h.open_session(&DotNetSessionConfig::default());
    a.invoke(st("System.Convert"), "ToInt32", vec![s("1")]).unwrap();
    wait_for_started(&h, 2); // the taken one was replaced
    a.close();
    h.shutdown();
    assert_eq!(h.stats().started, 2, "nothing starts after shutdown: {:?}", h.stats());
    assert_eq!(entries_of(work.path()), 0, "every working folder is gone");
    // After shutdown a session still works; it starts its own process.
    let mut b = h.open_session(&DotNetSessionConfig::default());
    b.invoke(st("System.Convert"), "ToInt32", vec![s("1")]).unwrap();
    assert_eq!(h.stats().from_standby, 1);
    b.close();
}

#[test]
fn a_pool_kept_for_first_use_starts_nothing_until_a_session_calls_then_keeps_standbys_ready() {
    let Some(h) = host() else { return };
    h.prewarm_on_first_use(1);
    // A run that never calls .NET starts no process at all.
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(h.stats().started, 0, "{:?}", h.stats());
    // The first session pays for its own start; a standby starts beside it for the next one.
    let mut a = h.open_session(&DotNetSessionConfig::default());
    a.invoke(st("System.Convert"), "ToInt32", vec![s("1")]).unwrap();
    assert_eq!(h.stats().from_standby, 0, "{:?}", h.stats());
    wait_for_started(&h, 2);
    let mut b = h.open_session(&DotNetSessionConfig::default());
    b.invoke(st("System.Convert"), "ToInt32", vec![s("1")]).unwrap();
    assert_eq!(h.stats().from_standby, 1, "{:?}", h.stats());
    a.close();
    b.close();
    h.shutdown();
}

#[test]
fn a_standby_that_cannot_start_leaves_the_session_to_report_the_reason() {
    let dir = tempfile::tempdir().unwrap();
    let dll = dir.path().join("AlDotNetSidecar.dll");
    std::fs::write(&dll, b"").unwrap();
    let script = "if [ \"$1\" = \"--list-runtimes\" ]; then echo 'Microsoft.NETCore.App 8.0.1 [/x]'; exit 0; fi\necho boom >&2\nexit 3";
    let h = SidecarHost::new(SidecarConfig {
        dotnet: Some(fake_dotnet(dir.path(), script)),
        sidecar: Some(dll),
        ..Default::default()
    });
    h.prewarm(2);
    let mut a = h.open_session(&DotNetSessionConfig::default());
    let why = refused(a.invoke(st("System.Convert"), "ToInt32", vec![s("1")]));
    assert!(why.contains("sidecar process ended") || why.contains("boom"), "{why}");
    assert_eq!(h.stats().from_standby, 0);
    a.close();
    h.shutdown();
}

// ---- ADR-024: CopyStream with a .NET stream ----------------------------------------------

#[test]
fn stream_write_appends_to_a_stream_and_stream_read_returns_the_whole_memory_stream() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let m = sess.construct(&ty("System.IO.MemoryStream"), vec![]).unwrap();
    let m = handle_of(&m);
    sess.stream_write(m, b"hello world").unwrap();
    let len = sess.get_property(DotNetTarget::Instance(m), "Position").unwrap();
    assert_eq!(len.value, DotNetValue::Int64(11), "the position is at the end");
    sess.invoke(
        DotNetTarget::Instance(m),
        "Seek",
        vec![arg(DotNetValue::Int64(6)), arg(DotNetValue::Int32(0))],
    )
    .unwrap();
    // Whatever the position, the whole stream; the position is 0 afterwards.
    assert_eq!(sess.stream_read(m, None).unwrap(), b"hello world");
    let pos = sess.get_property(DotNetTarget::Instance(m), "Position").unwrap();
    assert_eq!(pos.value, DotNetValue::Int64(0));
    // With a count: from the current position, which then follows the count.
    assert_eq!(sess.stream_read(m, Some(5)).unwrap(), b"hello");
    let pos = sess.get_property(DotNetTarget::Instance(m), "Position").unwrap();
    assert_eq!(pos.value, DotNetValue::Int64(5));
    sess.close();
}

#[test]
fn stream_ops_refuse_a_handle_that_is_not_a_stream_and_a_read_of_a_stream_that_is_not_a_memory_stream() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let b = sess.construct(&ty("System.Text.StringBuilder"), vec![]).unwrap();
    let why = refused_text(sess.stream_write(handle_of(&b), b"x"));
    assert!(
        why.contains("System.Text.StringBuilder") && why.contains("Stream"),
        "{why}"
    );
    let why = refused_text(sess.stream_read(handle_of(&b), None).map(|_| ()));
    assert!(why.contains("System.Text.StringBuilder"), "{why}");
    sess.close();
}

fn refused_text(r: Result<(), DotNetFailure>) -> String {
    match r {
        Err(DotNetFailure::Refused(s)) => s,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// ---- ADR-024 follow-up: a static member read through an object of a derived type ----------

#[test]
fn a_static_member_is_found_through_an_instance_of_a_derived_type() {
    // NPR `Encoding.UTF8.GetBytes(...)` on a variable that already holds an Encoding: the object's type is
    // `UTF8EncodingSealed`, and `UTF8` is a static member of its base class `Encoding`.
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let enc = sess.get_property(st("System.Text.Encoding"), "UTF8").unwrap();
    let enc = handle_of(&enc);
    let again = sess.get_property(DotNetTarget::Instance(enc), "UTF8").unwrap();
    assert!(matches!(again.value, DotNetValue::Handle { .. }), "{again:?}");
    sess.close();
}

#[test]
fn a_property_called_with_parentheses_reads_the_property() {
    // NPR CloudflareMediaImpl: `DotNetEncoding.UTF8();` AL lets a property be written as a call.
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let r = sess.invoke(st("System.Text.Encoding"), "UTF8", vec![]).unwrap();
    assert!(matches!(r.value, DotNetValue::Handle { .. }), "{r:?}");
    // With an argument it is still not a property.
    let why = refused(sess.invoke(st("System.Text.Encoding"), "UTF8", vec![s("x")]));
    assert!(why.contains("not found"), "{why}");
    sess.close();
}

#[test]
fn a_member_can_be_called_on_a_boxed_primitive() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    // 2022-01-01 in ticks since 0001-01-01.
    let ticks = 738_156i64 * 864_000_000_000;
    let r = sess
        .get_property(DotNetTarget::Value(DotNetValue::DateTime(ticks)), "Year")
        .unwrap();
    assert_eq!(r.value, DotNetValue::Int32(2022));
    let r = sess
        .invoke(DotNetTarget::Value(DotNetValue::Byte(7)), "ToString", vec![])
        .unwrap();
    assert_eq!(r.value, DotNetValue::Str("7".into()));
    sess.close();
}

#[test]
fn a_settable_property_called_with_one_argument_is_assigned() {
    // System Application "XML DOM Management".XMLEscape: `XMLNode.InnerText(Text); exit(XMLNode.InnerXml);` AL lets a
    // property be written as a call, with the value as the one argument. A read-only property stays refused.
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let doc = handle_of(&sess.construct(&ty("System.Xml.XmlDocument"), vec![]).unwrap());
    let el = handle_of(
        &sess
            .invoke(DotNetTarget::Instance(doc), "CreateElement", vec![s("E")])
            .unwrap(),
    );
    sess.invoke(DotNetTarget::Instance(el), "InnerText", vec![s("a<b & c")])
        .unwrap();
    let xml = sess.get_property(DotNetTarget::Instance(el), "InnerXml").unwrap();
    assert_eq!(xml.value, DotNetValue::Str("a&lt;b &amp; c".into()));
    let why = refused(sess.invoke(DotNetTarget::Instance(el), "LocalName", vec![s("x")]));
    assert!(why.contains("not found"), "{why}");
    sess.close();
}

/// AL spells a .NET indexer by the name the framework gives it (`IndexerName`), for the attribute
/// collection `ItemOf`; the Microsoft test apps call `Attributes.ItemOf('name')` and `ItemOf(0)`.
#[test]
fn an_indexer_is_called_by_its_indexer_name_with_its_arguments() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let doc = handle_of(&sess.construct(&ty("System.Xml.XmlDocument"), vec![]).unwrap());
    sess.invoke(DotNetTarget::Instance(doc), "LoadXml", vec![s("<r a=\"1\" b=\"2\"/>")])
        .unwrap();
    let root = handle_of(
        &sess
            .get_property(DotNetTarget::Instance(doc), "DocumentElement")
            .unwrap(),
    );
    let attrs = handle_of(&sess.get_property(DotNetTarget::Instance(root), "Attributes").unwrap());
    let by_name = handle_of(
        &sess
            .invoke(DotNetTarget::Instance(attrs), "ItemOf", vec![s("b")])
            .unwrap(),
    );
    let v = sess.get_property(DotNetTarget::Instance(by_name), "Value").unwrap();
    assert_eq!(text_of(&v), "2");
    let by_pos = handle_of(
        &sess
            .invoke(
                DotNetTarget::Instance(attrs),
                "ItemOf",
                vec![arg(DotNetValue::Int32(0))],
            )
            .unwrap(),
    );
    let v = sess.get_property(DotNetTarget::Instance(by_pos), "Value").unwrap();
    assert_eq!(text_of(&v), "1");
    sess.close();
}

/// AL assigns through a .NET indexer by calling it with the index arguments and the value last: System Application
/// "Business Chart Impl." writes `DataRow.Item(ColumnName, Value)` and reads `DataRow.Item(ColumnName)` back (corpus
/// tests/al-language/misc/lt-au-language-edges/AuEdgeTests.al:608 BusinessChartBuffer_Initialize_InTest, real BC 27-29:
/// SetValueByIndex(0, 0, 7) then GetValue answers 7).
#[test]
fn an_indexer_called_with_one_argument_more_than_its_index_is_assigned() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let table = handle_of(
        &sess
            .construct(&ty("System.Data.DataTable"), vec![s("DataTable")])
            .unwrap(),
    );
    let cols = handle_of(&sess.get_property(DotNetTarget::Instance(table), "Columns").unwrap());
    sess.invoke(DotNetTarget::Instance(cols), "Add", vec![s("Month")])
        .unwrap();
    let row = handle_of(&sess.invoke(DotNetTarget::Instance(table), "NewRow", vec![]).unwrap());
    sess.invoke(DotNetTarget::Instance(row), "Item", vec![s("Month"), s("Jan")])
        .unwrap();
    let v = sess
        .invoke(DotNetTarget::Instance(row), "Item", vec![s("Month")])
        .unwrap();
    assert_eq!(text_of(&v), "Jan");
    // An Integer value also fits the two-argument getter `Item(String, DataRowVersion)`; the setter is the closer fit.
    sess.invoke(
        DotNetTarget::Instance(row),
        "Item",
        vec![s("Month"), arg(DotNetValue::Int32(7))],
    )
    .unwrap();
    let v = sess
        .invoke(DotNetTarget::Instance(row), "Item", vec![s("Month")])
        .unwrap();
    assert_eq!(text_of(&v), "7");
    sess.close();
}

/// `foreach` over an XmlAttributeCollection with no attributes (Base App "XML DOM Management".AddNamespaces, al-runner
/// issue 6316): GetEnumerator returns System.Xml.EmptyEnumerator, which implements IEnumerator.MoveNext and Current only
/// explicitly. The loop calls them by name, so they answer through the IEnumerator interface: MoveNext is false.
#[test]
fn move_next_and_current_answer_through_ienumerator_on_an_explicit_implementation() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let doc = handle_of(&sess.construct(&ty("System.Xml.XmlDocument"), vec![]).unwrap());
    sess.invoke(DotNetTarget::Instance(doc), "LoadXml", vec![s("<r/>")])
        .unwrap();
    let root = handle_of(
        &sess
            .get_property(DotNetTarget::Instance(doc), "DocumentElement")
            .unwrap(),
    );
    let attrs = handle_of(&sess.get_property(DotNetTarget::Instance(root), "Attributes").unwrap());
    let en = handle_of(
        &sess
            .invoke(DotNetTarget::Instance(attrs), "GetEnumerator", vec![])
            .unwrap(),
    );
    let more = sess.invoke(DotNetTarget::Instance(en), "MoveNext", vec![]).unwrap();
    assert_eq!(more.value, DotNetValue::Bool(false));
    sess.close();
}

// A generic type AL declares without type arguments (System Application's ABS/AFS Operation Payload declares
// `GenericSortedDictionary2` and calls `SortedDictionary.SortedDictionary()`): BC closes it over System.Object, as the closed
// type name in its messages shows (tests/al-language-onprem-dotnet/lt-y17-dotnet-dictionary/Y17DotNetDictionary.al:126, :147).
#[test]
fn an_open_generic_type_is_constructed_closed_over_object() {
    let Some(h) = host() else { return };
    let mut sess = h.open_session(&DotNetSessionConfig::default());
    let d = sess
        .construct(&ty("System.Collections.Generic.SortedDictionary`2"), vec![])
        .unwrap();
    assert!(
        matches!(&d.value, DotNetValue::Handle { ty, .. } if ty.starts_with("System.Collections.Generic.SortedDictionary`2[[System.Object")),
        "{:?}",
        d.value
    );
    let d = handle_of(&d);
    sess.invoke(DotNetTarget::Instance(d), "Add", vec![s("b"), s("2")])
        .unwrap();
    sess.invoke(DotNetTarget::Instance(d), "Add", vec![s("a"), s("1")])
        .unwrap();
    let n = sess.get_property(DotNetTarget::Instance(d), "Count").unwrap();
    assert_eq!(n.value, DotNetValue::Int32(2));
}

/// The helper built from `tools/dotnet-sidecar` is ReadyToRun (the csproj's `AlReadyToRun`): every DotNet session starts
/// a sidecar process, which would otherwise JIT-compile the dispatch code each time (about 1 % of a full BC 29 run's
/// instructions). A ReadyToRun image carries the `RTR` signature of its ReadyToRun header; a JIT-only build does not.
/// The ReadyToRun packs come from NuGet: a machine without access to them gets the plain build, and this test fails there.
#[test]
fn the_sidecar_built_from_source_is_ready_to_run() {
    let Some(h) = host() else { return };
    let dll = h.sidecar_path().expect("the host resolved its sidecar");
    let bytes = std::fs::read(&dll).unwrap();
    assert!(
        bytes.windows(4).any(|w| w == b"RTR\0"),
        "{} is not a ReadyToRun image",
        dll.display()
    );
}
