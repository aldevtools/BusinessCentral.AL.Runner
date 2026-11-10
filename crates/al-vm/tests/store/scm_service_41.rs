//! Refusals that kept Microsoft's Tests-SCM-Service tests from running (issue 41).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_vm::{Engine, Session};

const TABLES: &str = r#"
table 50800 "SV Row"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Name; Text[50]) { }
        field(3; Qty; Decimal) { }
    }
    keys { key(PK; "Entry No.") { } }
}
"#;

fn text(body: &str) -> Result<String, Signal> {
    let src =
        format!("{TABLES}codeunit 50100 C {{ procedure F(): Text var Rec: Record \"SV Row\"; begin {body} end; }}");
    let l = al_testkit::compile(&src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.routines.iter().any(|r| r.name == "F"))
        .expect("object");
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    match s.call(oi, ri, inst, vec![])?.0 {
        Value::Text(t) => Ok(t.to_string()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn test_field_with_an_error_info_that_holds_does_nothing() {
    // Base Application "Gen. Jnl.-Check Line".RunCheck: `GenJnlLine.TestField("Account No.", ErrorInfo.Create())`
    // and `TestField(Blocked, false, ErrorInfo.Create())`. Documentation only: the check is TestField's,
    // and a check that holds raises nothing (docs/probes-needed.md).
    let r = text(
        "Rec.\"Entry No.\" := 1; Rec.Name := 'x'; Rec.TestField(Name, ErrorInfo.Create());
         Rec.TestField(Name, 'x', ErrorInfo.Create()); Rec.TestField(Qty, 0, ErrorInfo.Create('m')); exit('ok');",
    );
    assert_eq!(r.unwrap(), "ok");
}

#[test]
fn test_field_with_an_error_info_that_fails_raises_the_plain_test_field_error() {
    // types/PUTestFieldErrorInfo.al (BC 27.5 and 29.0): the same text and code as the form without an ErrorInfo,
    // whatever Message the ErrorInfo carries.
    for (body, text_start) in [
        (
            "Rec.TestField(Name, ErrorInfo.Create('custom')); exit('ran');",
            "Name must have a value in",
        ),
        (
            "Rec.TestField(Name, 'y', ErrorInfo.Create()); exit('ran');",
            "Name must be equal to 'y'",
        ),
    ] {
        match text(body) {
            Err(Signal::Error(e)) => {
                assert!(e.message.starts_with(text_start), "{}", e.message);
                assert_eq!(e.code, "TestField");
            }
            other => panic!("{body}: {other:?}"),
        }
    }
}
