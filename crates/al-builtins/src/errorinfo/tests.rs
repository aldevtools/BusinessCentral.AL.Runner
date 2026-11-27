//! `ErrorInfo.AddAction` with the method `Run` against a stub session: which codeunit shapes raise, take the action or
//! refuse (assumption `errorinfo-add-action-run`, corpus PR 1601 EnTErrorInfoRunAction.al).

use super::{add_action, add_run_action};
use crate::native::tests::TestCtx;
use crate::native::{CodeunitShape, Signal};
use crate::value::Value;

fn run_action(shape: Option<CodeunitShape>, codeunit: i32, method: &str) -> Result<(), Signal> {
    let mut c = TestCtx::default();
    c.codeunit_shape = shape;
    add_run_action(&c, codeunit, method)
}

fn refusal(r: Result<(), Signal>) -> String {
    match r {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn run_on_a_normal_codeunit_raises_the_unknown_type_error() {
    match run_action(Some(CodeunitShape::Normal), 50100, "Run") {
        Err(Signal::Error(e)) => {
            assert_eq!(e.message, "The type Microsoft.Dynamics.Nav.Types.DataError is unknown.");
            assert_eq!(e.code, "NotSupportedType");
        }
        other => panic!("expected the unknown-type error, got {other:?}"),
    }
}

#[test]
fn run_on_codeunit_zero_or_a_missing_id_takes_the_action() {
    assert!(run_action(None, 0, "Run").is_ok());
    assert!(run_action(Some(CodeunitShape::Missing), 50100, "Run").is_ok());
}

#[test]
fn other_spellings_negative_ids_and_other_shapes_refuse() {
    let upper = refusal(run_action(Some(CodeunitShape::Normal), 50100, "RUN"));
    assert!(
        upper.starts_with("ErrorInfo.AddAction with the method 'RUN' on codeunit 50100"),
        "{upper}"
    );
    let negative = refusal(run_action(Some(CodeunitShape::Normal), -1, "Run"));
    assert!(
        negative.starts_with("ErrorInfo.AddAction with the method 'Run' on codeunit -1"),
        "{negative}"
    );
    let other = refusal(run_action(
        Some(CodeunitShape::Other("a codeunit that declares a Subtype")),
        50100,
        "Run",
    ));
    assert!(
        other.contains("on codeunit 50100, a codeunit that declares a Subtype"),
        "{other}"
    );
}

#[test]
fn add_action_with_run_raises_before_it_touches_the_error_info() {
    let mut c = TestCtx::default();
    c.codeunit_shape = Some(CodeunitShape::Normal);
    let mut args = [
        Value::Text("Fix it".to_string().into()),
        Value::Integer(50100),
        Value::Text("Run".to_string().into()),
    ];
    assert!(matches!(add_action(&mut c, None, &mut args), Err(Signal::Error(_))));
}

#[test]
fn a_session_without_a_program_refuses_run() {
    let why = refusal(run_action(None, 50100, "Run"));
    assert!(why.contains("on codeunit 50100, no program in this session"), "{why}");
}
