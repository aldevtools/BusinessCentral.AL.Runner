//! What a failed test under TestIsolation = Codeunit takes back of the number sequences
//! (`Session::drop_sequences_inserted_since`; tests/al-language-isolation-probe/src/IsolProbeAfterRollback.Codeunit.al
//! S3, S4, S8, S9 under runner 61301).

use al_testkit::Built;

const SRC: &str = r#"
codeunit 50100 Seq
{
    procedure Make(Name: Text)
    begin
        NumberSequence.Insert(Name, 1, 1);
    end;

    procedure Take(Name: Text): BigInteger
    begin
        exit(NumberSequence.Next(Name));
    end;

    procedure Restart(Name: Text)
    begin
        NumberSequence.Restart(Name, 1);
    end;

    procedure Gone(Name: Text): Boolean
    begin
        exit(not NumberSequence.Exists(Name));
    end;
}
"#;

fn call(b: &mut Built, routine: &str, name: &str) -> al_builtins::value::Value {
    let (oi, ri) = b.routine_in("Seq", routine);
    b.call(oi, ri, vec![al_builtins::value::Value::Text(name.into())])
        .unwrap()
        .0
}

fn gone(b: &mut Built, name: &str) -> bool {
    matches!(call(b, "Gone", name), al_builtins::value::Value::Boolean(true))
}

#[test]
fn a_sequence_inserted_since_the_start_is_gone_and_numbers_taken_from_an_older_one_stay() {
    let mut b = Built::from_source(SRC);
    call(&mut b, "Make", "OLD");
    let start = b.session.sequences_at_start();
    call(&mut b, "Take", "OLD");
    call(&mut b, "Make", "NEW");
    call(&mut b, "Take", "NEW");
    assert_eq!(b.session.drop_sequences_inserted_since(&start), Some(true));
    assert!(gone(&mut b, "NEW"));
    assert!(!gone(&mut b, "OLD"));
    // The number OLD gave out is not given back.
    assert!(matches!(
        call(&mut b, "Take", "OLD"),
        al_builtins::value::Value::BigInteger(2)
    ));
    assert_eq!(b.session.drop_sequences_inserted_since(&start), None);
}

#[test]
fn a_restart_of_an_older_sequence_is_not_measured() {
    let mut b = Built::from_source(SRC);
    call(&mut b, "Make", "OLD");
    call(&mut b, "Take", "OLD");
    let start = b.session.sequences_at_start();
    call(&mut b, "Restart", "OLD");
    call(&mut b, "Take", "OLD");
    assert_eq!(b.session.drop_sequences_inserted_since(&start), Some(false));
}
