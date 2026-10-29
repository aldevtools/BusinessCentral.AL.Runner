//! AL source to the real .NET sidecar, through the whole runtime path (lexer, sema, lowering,
//! VM, marshalling, process, reflection). Own fixtures: an OnPrem-style `dotnet { }` block with
//! System.Convert, System.Text.Encoding and System.IO.MemoryStream, written the way Microsoft's
//! Base64 Convert Impl uses them (docs/dotnet-examples.md, case 1). Needs .NET 8; without it a
//! test skips visibly, and `ALRT_REQUIRE_DOTNET=1` turns a skip into a failure.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_host::dotnet::DotNetHost;
use al_sema::workspace::AppSource;
use al_test_cli::dotnet::{SidecarConfig, SidecarHost};
use al_vm::dotnet::DotNetConfig;
use al_vm::{Engine, Session};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

const SRC: &str = r#"
dotnet
{
    assembly("netstandard")
    {
        Culture = 'neutral';
        PublicKeyToken = 'cc7b13ffcd2ddd51';
        type("System.Convert"; "Convert") { }
        type("System.Text.Encoding"; "Encoding") { }
        type("System.IO.MemoryStream"; "MemoryStream") { }
        type("System.Array"; "Array") { }
        type("System.IO.File"; "File") { }
        type("System.Collections.Generic.List`1[[System.String, mscorlib]]"; "StringList") { }
    }
}

table 60008 "ALT Blob"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(3; Data; Blob) { }
    }
    keys { key(PK; "Code") { } }
}

codeunit 50100 C
{
    procedure ToBase64(Value: Text): Text
    var
        Convert: DotNet Convert;
        Encoding: DotNet Encoding;
        Bytes: DotNet Array;
    begin
        Encoding := Encoding.UTF8;
        Bytes := Encoding.GetBytes(Value);
        exit(Convert.ToBase64String(Bytes));
    end;

    procedure FromBase64(Base64: Text; var OutStr: OutStream)
    var
        Convert: DotNet Convert;
        MemoryStream: DotNet MemoryStream;
    begin
        MemoryStream := MemoryStream.MemoryStream(Convert.FromBase64String(Base64));
        MemoryStream.WriteTo(OutStr);
    end;

    procedure RoundTripThroughBlob(Base64: Text): Text
    var
        B: Record "ALT Blob";
        O: OutStream;
        I: InStream;
        T: Text;
    begin
        B.Code := 'X';
        B.Data.CreateOutStream(O);
        FromBase64(Base64, O);
        B.Insert();
        B.Get('X');
        B.CalcFields(Data);
        B.Data.CreateInStream(I);
        I.ReadText(T);
        exit(T);
    end;

    procedure StreamProperties(): Text
    var
        MemoryStream: DotNet MemoryStream;
        Length: BigInteger;
        Position: BigInteger;
    begin
        MemoryStream := MemoryStream.MemoryStream();
        MemoryStream.WriteByte(65);
        MemoryStream.WriteByte(66);
        Length := MemoryStream.Length;
        MemoryStream.Position := 1;
        Position := MemoryStream.Position;
        exit(Format(Length) + ':' + Format(Position));
    end;

    procedure BadBase64(): Integer
    var
        Convert: DotNet Convert;
        Bytes: DotNet Array;
    begin
        Bytes := Convert.FromBase64String('not base64!');
        exit(Bytes.Length);
    end;

    procedure ListCount(): Integer
    var
        List: DotNet StringList;
    begin
        List := List.StringList();
        List.Add('a');
        List.Add('b');
        exit(List.Count);
    end;

    procedure TouchesTheFileSystem(): Boolean
    var
        File: DotNet File;
    begin
        exit(File.Exists('/etc/passwd'));
    end;

    procedure ToInt(Value: Text): Integer
    var
        Convert: DotNet Convert;
    begin
        exit(Convert.ToInt32(Value));
    end;

    procedure Loop(N: Integer): Integer
    var
        Convert: DotNet Convert;
        I: Integer;
        Sum: Integer;
    begin
        for I := 1 to N do
            Sum += Convert.ToInt32('3');
        exit(Sum);
    end;
}
"#;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn host() -> Option<Arc<SidecarHost>> {
    let h = Arc::new(SidecarHost::new(SidecarConfig {
        source_root: Some(root()),
        cache_dir: Some(Path::new(env!("CARGO_TARGET_TMPDIR")).join("dotnet-sidecar-cache")),
        ..Default::default()
    }));
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

struct Fixture {
    program: std::sync::Arc<al_ir::Program>,
    blocked: Vec<Vec<Option<String>>>,
    oi: usize,
}

fn fixture() -> Fixture {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), SRC.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    Fixture {
        program: l.program,
        blocked,
        oi,
    }
}

impl Fixture {
    fn session(&self, host: Option<Arc<dyn DotNetHost>>) -> Session {
        let mut cfg = DotNetConfig::default();
        if let Some(h) = host {
            cfg.host = h;
        }
        Session::new(Engine::with_dotnet(self.program.clone(), self.blocked.clone(), cfg))
    }

    fn call(&self, s: &mut Session, name: &str, args: Vec<Value>) -> Result<Value, Signal> {
        let ri = self.program.objects[self.oi]
            .routines
            .iter()
            .position(|r| r.name == name)
            .unwrap_or_else(|| panic!("no routine {name}"));
        if let Some(Some(r)) = self.blocked[self.oi].get(ri) {
            return Err(Signal::Unsupported(format!("lowering: {r}")));
        }
        let inst = s.instance(self.oi);
        s.call(self.oi, ri, inst, args).map(|(v, _)| v)
    }
}

fn text(v: Result<Value, Signal>) -> String {
    match v {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

fn why(v: Result<Value, Signal>) -> String {
    match v {
        Err(Signal::Unsupported(s)) => s,
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn the_fixture_lowers_without_any_refusal_except_the_ones_it_means() {
    let f = fixture();
    let reasons: Vec<(String, Option<String>)> = f.program.objects[f.oi]
        .routines
        .iter()
        .enumerate()
        .map(|(i, r)| (r.name.clone(), f.blocked[f.oi][i].clone()))
        .collect();
    for (name, r) in reasons {
        assert!(r.is_none(), "{name} is refused statically: {r:?}");
    }
}

#[test]
fn base64_encoding_runs_on_the_real_framework() {
    let Some(h) = host() else { return };
    let f = fixture();
    let mut s = f.session(Some(h));
    assert_eq!(
        text(f.call(&mut s, "ToBase64", vec![Value::Text("hello".into())])),
        "aGVsbG8="
    );
    assert_eq!(
        text(f.call(&mut s, "ToBase64", vec![Value::Text("héllo wörld".into())])),
        "aMOpbGxvIHfDtnJsZA=="
    );
}

#[test]
fn base64_decoding_writes_the_bytes_into_an_al_blob_through_an_outstream_bridge() {
    // Microsoft's Base64 Convert Impl.FromBase64 does exactly this (docs/dotnet-examples.md case 1).
    let Some(h) = host() else { return };
    let f = fixture();
    let mut s = f.session(Some(h));
    assert_eq!(
        text(f.call(
            &mut s,
            "RoundTripThroughBlob",
            vec![Value::Text("aGVsbG8gd29ybGQ=".into())]
        )),
        "hello world"
    );
}

#[test]
fn a_constructed_stream_has_properties_that_can_be_read_and_written() {
    let Some(h) = host() else { return };
    let f = fixture();
    let mut s = f.session(Some(h));
    assert_eq!(text(f.call(&mut s, "StreamProperties", vec![])), "2:1");
}

#[test]
fn a_generic_list_declared_with_the_clr_spelling_works() {
    let Some(h) = host() else { return };
    let f = fixture();
    let mut s = f.session(Some(h));
    assert!(matches!(f.call(&mut s, "ListCount", vec![]), Ok(Value::Integer(2))));
}

#[test]
fn a_dotnet_exception_is_not_run_and_carries_the_clr_type_and_message() {
    let Some(h) = host() else { return };
    let f = fixture();
    let mut s = f.session(Some(h));
    let w = why(f.call(&mut s, "BadBase64", vec![]));
    assert!(w.contains("System.Convert.FromBase64String"), "{w}");
    assert!(w.contains("System.FormatException"), "{w}");
    assert!(w.contains("not pinned"), "{w}");
    // The sidecar is still there for the next call.
    assert!(matches!(
        f.call(&mut s, "ToInt", vec![Value::Text("21".into())]),
        Ok(Value::Integer(21))
    ));
}

#[test]
fn file_exists_answers_from_the_in_memory_file_system_never_the_disk() {
    // ADR-056: .NET file calls run against the run's in-memory file system. `/etc/passwd` exists on the
    // disk of every Linux machine, so `false` shows the call never reached the real disk.
    let Some(h) = host() else { return };
    let f = fixture();
    let mut s = f.session(Some(h));
    match f.call(&mut s, "TouchesTheFileSystem", vec![]) {
        Ok(Value::Boolean(false)) => {}
        other => panic!("expected Ok(Boolean(false)), got {other:?}"),
    }
}

#[test]
fn without_a_usable_dotnet_the_call_is_refused_with_the_unavailable_marker() {
    let f = fixture();
    let h = Arc::new(SidecarHost::new(SidecarConfig {
        sidecar: Some(PathBuf::from("/nonexistent/AlDotNetSidecar.dll")),
        ..Default::default()
    }));
    let mut s = f.session(Some(h));
    let w = why(f.call(&mut s, "ToBase64", vec![Value::Text("x".into())]));
    assert!(w.starts_with("dotnet-unavailable: DotNet "), "{w}");
}

#[test]
fn each_session_gets_its_own_process_so_dotnet_statics_do_not_leak() {
    let Some(h) = host() else { return };
    let f = fixture();
    let h: Arc<dyn DotNetHost> = h;
    let mut a = f.session(Some(h.clone()));
    let mut b = f.session(Some(h));
    assert!(matches!(
        f.call(&mut a, "ToInt", vec![Value::Text("1".into())]),
        Ok(Value::Integer(1))
    ));
    assert!(matches!(
        f.call(&mut b, "ToInt", vec![Value::Text("2".into())]),
        Ok(Value::Integer(2))
    ));
}

/// Timing of a DotNet call through the full runtime path. Run in release:
/// `cargo test --release -p al-test-cli --test dotnet_runtime -- --ignored --nocapture timing`
#[test]
#[ignore]
fn timing_of_dotnet_calls_through_the_full_runtime_path() {
    let Some(h) = host() else { return };
    let f = fixture();
    // Warm the sidecar cache build so it is not part of the numbers.
    let mut s = f.session(Some(h.clone()));
    let t = Instant::now();
    let v = f.call(&mut s, "ToInt", vec![Value::Text("1".into())]);
    let first = t.elapsed();
    assert!(matches!(v, Ok(Value::Integer(1))));
    let t = Instant::now();
    f.call(&mut s, "ToInt", vec![Value::Text("1".into())]).unwrap();
    let second = t.elapsed();
    let t = Instant::now();
    f.call(&mut s, "ToBase64", vec![Value::Text("hello".into())]).unwrap();
    let chain = t.elapsed();
    const N: i32 = 2000;
    let t = Instant::now();
    let v = f.call(&mut s, "Loop", vec![Value::Integer(N)]);
    let loop_time = t.elapsed();
    assert!(matches!(v, Ok(Value::Integer(n)) if n == 3 * N));
    let t = Instant::now();
    drop(s);
    let close = t.elapsed();
    eprintln!("first call incl. process start (cold session): {first:?}");
    eprintln!("second call (warm): {second:?}");
    eprintln!("ToBase64 (5 .NET calls: UTF8, GetBytes, ToBase64String + conversions): {chain:?}");
    eprintln!(
        "{N} x Convert.ToInt32 in an AL loop: {loop_time:?} = {:?} per call",
        loop_time / N as u32
    );
    eprintln!("session close: {close:?}");
}
