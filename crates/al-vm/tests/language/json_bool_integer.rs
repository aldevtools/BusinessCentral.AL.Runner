//! `JsonArray.GetInteger` of a JSON `true` reads 1 (json/PnAJsonGetters.al:191-210,
//! ArrayWrongKindAtIndex `int-of-bool`).

use al_builtins::native::Signal;

fn run(json: &str) -> Result<String, Signal> {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var JA: JsonArray; begin JA.ReadFrom('{json}'); exit(Format(JA.GetInteger(0))); end; }}"
    );
    al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| al_testkit::text(Ok(v)))
}

#[test]
fn get_integer_of_true_is_one() {
    assert_eq!(run("[true]").unwrap(), "1");
}

#[test]
fn get_integer_of_false_is_refused() {
    match run("[false]") {
        Err(Signal::Unsupported(_)) => {}
        other => panic!("{other:?}"),
    }
}
