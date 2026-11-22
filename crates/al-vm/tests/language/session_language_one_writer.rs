//! `Session::set_language` is the one writer of the session language and of every copy the builtins read (al-runner
//! issue 7659): `GlobalLanguage()` in AL code, the Turkish `i` rule of Code conversions and the language a new field filter
//! records all follow it, and `restore_carried` goes through it too.

use al_builtins::value::Value;
use al_testkit::Built;

const SRC: &str = r#"
codeunit 50991 L
{
    procedure Lang(): Integer
    begin
        exit(GlobalLanguage());
    end;
}
"#;

fn al_language(b: &mut Built) -> i32 {
    let (oi, ri) = b.routine_in("L", "Lang");
    match b.call(oi, ri, Vec::new()).map(|(v, _)| v) {
        Ok(Value::Integer(n)) => n,
        other => panic!("GlobalLanguage() gave {other:?}"),
    }
}

#[test]
fn set_language_updates_every_copy() {
    let mut b = Built::from_source(SRC);
    b.session.set_language(1055);
    assert_eq!(al_language(&mut b), 1055);
    assert!(
        al_builtins::ops::turkic_code(),
        "Turkish sets the Turkish i rule of Code conversions"
    );
    assert_eq!(al_builtins::filter::filter_language(), 1055);

    b.session.set_language(1033);
    assert_eq!(al_language(&mut b), 1033);
    assert!(!al_builtins::ops::turkic_code());
    assert_eq!(al_builtins::filter::filter_language(), 1033);
}

#[test]
fn restore_carried_goes_through_the_one_writer() {
    let mut b = Built::from_source(SRC);
    b.session.set_language(1055);
    let carried = b.session.carried_state();
    b.session.set_language(1031);
    assert_eq!(al_builtins::filter::filter_language(), 1031);
    b.session.restore_carried(carried);
    assert_eq!(al_language(&mut b), 1055);
    assert!(al_builtins::ops::turkic_code());
    assert_eq!(al_builtins::filter::filter_language(), 1055);
}
