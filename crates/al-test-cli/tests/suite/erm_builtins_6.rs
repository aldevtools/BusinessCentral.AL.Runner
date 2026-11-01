//! Tests-ERM refusals of issue 26: an Integer stored into an Enum and `RecordRef.Get(RecordId)` on a
//! RecordRef that is not open. `RecordRef.GetBySystemId`, refused here before, is probed now (al-vm
//! tests/record_builtins.rs).

use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

fn compile(tests: &str) -> (al_sema::lower::Lowered, usize) {
    let src = format!(
        "enum 62900 \"B6 Kind\" {{ value(0; A) {{ }} value(1; B) {{ }} }}
         table 62901 \"B6 Row\" {{ fields {{ field(1; Kind; Enum \"B6 Kind\") {{ }} field(2; Name; Text[20]) {{ }} }} keys {{ key(PK; Kind) {{ }} }} }}
         codeunit 62902 T {{ Subtype = Test; {tests} }}"
    );
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    (l, oi)
}

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let (l, oi) = compile(tests);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    run_codeunit(&engine, &l, oi)
}

fn not_run(o: &(String, Outcome)) -> String {
    match &o.1 {
        Outcome::NotRun(why) => why.clone(),
        other => panic!("{}: {other:?}", o.0),
    }
}

// An Integer key for an Enum field is named as the key's enum value in the not-found error
// (corpus errors/lt-ms-refusals-138/P138EnumGetNotFound.al:126-130).
#[test]
fn an_integer_key_for_an_enum_field_names_the_enum_value_when_not_found() {
    let o =
        outcomes("[Test] procedure IntegerKey() var R: Record \"B6 Row\"; N: Integer; begin N := 1; R.Get(N); end;");
    match &o[0].1 {
        Outcome::Fail(m) => {
            assert_eq!(
                m,
                "The B6 Row does not exist. Identification fields and values: Kind='B'"
            )
        }
        other => panic!("{other:?}"),
    }
}

// Get(RecordId) on a RecordRef never opened opens the RecordId's table (branch ms-top-refusals-1): the row is then looked up.
#[test]
fn get_by_record_id_on_a_closed_record_ref_opens_the_table_of_the_record_id() {
    let o =
        outcomes("[Test] procedure Closed() var R: Record \"B6 Row\"; RR: RecordRef; begin RR.Get(R.RecordId); end;");
    match &o[0].1 {
        Outcome::Fail(m) => assert!(m.contains("does not exist"), "{m}"),
        other => panic!("{other:?}"),
    }
}

/// `Database.SID()` without a user name: the probe (corpus branch al-runtime-rs-staging, ae3fe88,
/// tests/al-language-onprem/session/PXDatabaseSid.al) passes a name every time, so whose SID the
/// call without one answers is not pinned. Refused when it runs, with its reason.
#[test]
fn database_sid_without_a_user_name_is_refused_with_its_reason() {
    let o = outcomes("[Test] procedure NoName() var T: Text; begin T := Database.SID(); end;");
    let why = not_run(&o[0]);
    assert!(why.contains("Database.SID") && why.contains("not pinned"), "{why}");
}
