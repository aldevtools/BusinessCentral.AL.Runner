//! Builtins that kept Tests-ERM tests from running (issue 8): each test names the corpus test that
//! pins what it asserts, or says that nothing does and the builtin is refused with a reason.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_testkit::text;
use al_vm::{Engine, Session};

const SRC: &str = r#"
table 50900 "ALT T"
{
    fields { field(1; Id; Integer) { } field(2; Descr; Text[30]) { } }
    keys { key(PK; Id) { } }
}
report 50200 "ALT R" { ProcessingOnly = true; }
"#;

fn compile(body: &str) -> al_sema::lower::Lowered {
    let src = format!("{SRC}\ncodeunit 50100 C {{ {body} }}");
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
    l
}

/// Runs routine `F` of codeunit `C`.
fn run(body: &str) -> Result<Value, Signal> {
    let l = compile(body);
    let blocked = l
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
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

#[test]
fn http_request_message_method_round_trips() {
    // http/TestHttpRequestMessage.al:19-25 (`Req.Method := 'GET'`, read back); :46-55 the request uri.
    let r = run("procedure F(): Text var Req: HttpRequestMessage; begin Req.Method := 'POST'; exit(Req.Method); end;");
    assert_eq!(text(r), "POST");
    let r = run(
        "procedure F(): Text var Req: HttpRequestMessage; begin Req.Method := 'GET'; Req.Method := 'PUT'; exit(Req.Method); end;",
    );
    assert_eq!(text(r), "PUT");
    let r = run(
        "procedure F(): Text var Req: HttpRequestMessage; begin Req.SetRequestUri('https://example.invalid/probe'); exit(Req.GetRequestUri()); end;",
    );
    assert_eq!(text(r), "https://example.invalid/probe");
}

#[test]
fn http_request_message_method_nobody_set_is_get() {
    // types/TestHttpRequestFileUploadFieldErrorContracts.al:HttpRequestMessage_Defaults_AndSetRequestUri (BC 27.5).
    let r = run("procedure F(): Text var Req: HttpRequestMessage; begin exit(Req.Method); end;");
    assert_eq!(text(r), "GET");
    let r = run("procedure F(): Text var Req: HttpRequestMessage; begin exit(Req.GetRequestUri()); end;");
    assert_eq!(text(r), "");
}

#[test]
fn set_permission_filter_does_not_throw_and_changes_no_row() {
    // record/TestRecordFilter.al:406-413 pins that Record.SetPermissionFilter does not throw. Permission
    // enforcement is not modelled (docs/probes-needed.md), so the user can read every row: no row goes.
    let r = run(
        "procedure F(): Integer var R: Record \"ALT T\"; begin R.Id := 1; R.Insert(); R.Id := 2; R.Insert(); R.SetPermissionFilter(); exit(R.Count()); end;",
    );
    assert!(matches!(r, Ok(Value::Integer(2))), "{r:?}");
    let r = run(
        "procedure F(): Integer var R: Record \"ALT T\"; RR: RecordRef; begin R.Id := 1; R.Insert(); RR.Open(50900); RR.SetPermissionFilter(); exit(RR.Count()); end;",
    );
    assert!(matches!(r, Ok(Value::Integer(1))), "{r:?}");
}

#[test]
fn builtins_no_corpus_test_pins_are_refused_with_a_reason_that_says_so() {
    // Preview is pinned only inside a report run by Run or RunModal (handlers/
    // TestReportRequestPageUpdateObjectIdContracts.al); asked of a variable outside a run it refuses when it runs.
    match run("procedure F() var Rep: Report \"ALT R\"; begin Rep.Preview(); end;") {
        Err(Signal::Unsupported(why)) => {
            assert!(why.contains("Report.Preview"), "{why}");
            assert!(why.contains("not pinned"), "{why}");
        }
        other => panic!("{other:?}"),
    }
}
