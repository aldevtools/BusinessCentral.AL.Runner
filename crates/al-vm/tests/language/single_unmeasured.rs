//! al-runner issue 8239: a SingleInstance codeunit marked as holding state whose BC value the run does not know refuses
//! every read of one of its globals, from any code, and runs a write (`single_unmeasured.rs`). The mark moves with the
//! instance to a later session and goes when the session drops its single instances. A single instance marked as holding
//! BC's value by an assumption entry marks the entry on every read and keeps a refusal mark it already has. A session whose number sequences
//! are marked refuses every NumberSequence call (`storage_checkpoint.rs`).

use al_builtins::native::Signal;
use al_testkit::Built;

const SRC: &str = r#"
codeunit 50490 "Unm State"
{
    SingleInstance = true;
    var On: Boolean; Log: List of [Integer];
    procedure Put() begin On := true; end;
    procedure IsOn(): Boolean begin exit(On); end;
    procedure Add() begin Log.Add(1); end;
}
codeunit 50491 "Unm Driver"
{
    procedure Put() var S: Codeunit "Unm State"; begin S.Put(); end;
    procedure Read(): Boolean var S: Codeunit "Unm State"; begin exit(S.IsOn()); end;
    procedure Add() var S: Codeunit "Unm State"; begin S.Add(); end;
    procedure Seq(): BigInteger begin exit(NumberSequence.Current('UNMSEQ')); end;
    procedure Ins() begin NumberSequence.Insert('UNMSEQ2', 1, 1); end;
}
"#;

const WHY: &str = "the login changed it";

fn marked() -> Built {
    let mut b = Built::from_source(SRC);
    b.run_in("Unm Driver", "Put").expect("made");
    let object = b.routine_in("Unm State", "Put").0;
    b.session.mark_single_unmeasured(object, WHY.into());
    b
}

fn refused(r: Result<al_builtins::value::Value, Signal>) -> bool {
    matches!(r, Err(Signal::Unsupported(why)) if why.reason() == WHY)
}

#[test]
fn a_read_of_a_marked_single_instance_is_refused_and_a_write_runs() {
    let mut b = marked();
    b.run_in("Unm Driver", "Put").expect("a write runs");
    assert!(refused(b.run_in("Unm Driver", "Read")));
    // `Log.Add` reaches the list through a read of the global.
    assert!(refused(b.run_in("Unm Driver", "Add")));
}

#[test]
fn the_mark_moves_with_the_instance_and_goes_when_the_session_drops_it() {
    let mut b = marked();
    let moved = b.session.take_single_instances();
    b.session.adopt_single_instances(moved);
    assert!(refused(b.run_in("Unm Driver", "Read")), "moved on");
    b.session.reset_single_instances();
    assert!(b.run_in("Unm Driver", "Read").is_ok(), "a fresh instance");
}

#[test]
fn an_unmarked_single_instance_reads_as_before() {
    let mut b = Built::from_source(SRC);
    b.run_in("Unm Driver", "Put").expect("made");
    assert!(b.run_in("Unm Driver", "Read").is_ok());
}

#[test]
fn a_session_with_marked_number_sequences_refuses_every_call() {
    let mut b = Built::from_source(SRC);
    b.session.mark_numbers_unmeasured(WHY.into());
    assert!(refused(b.run_in("Unm Driver", "Seq")));
    assert!(refused(b.run_in("Unm Driver", "Ins")));
    b.session.clear_unmeasured();
    assert!(refused(b.run_in("Unm Driver", "Seq")), "it holds for the session");
}

use al_builtins::assumptions::{Assumption, take_hits};

/// A made single instance marked as holding BC's value by assumption `partner-login-memo-kept`.
fn assumed() -> Built {
    let mut b = Built::from_source(SRC);
    b.run_in("Unm Driver", "Put").expect("made");
    let object = b.routine_in("Unm State", "Put").0;
    b.session.mark_single_assumed(object, Assumption::PartnerLoginMemoKept);
    take_hits();
    b
}

#[test]
fn a_read_of_an_assumed_single_instance_marks_the_entry_and_a_write_does_not() {
    let mut b = assumed();
    b.run_in("Unm Driver", "Put").expect("a write runs");
    assert!(take_hits().is_empty(), "a write reads nothing");
    assert!(matches!(
        b.run_in("Unm Driver", "Read"),
        Ok(al_builtins::value::Value::Boolean(true))
    ));
    assert_eq!(take_hits(), [Assumption::PartnerLoginMemoKept]);
}

#[test]
fn an_assumed_mark_does_not_replace_a_refusal() {
    let mut b = marked();
    let object = b.routine_in("Unm State", "Put").0;
    b.session.mark_single_assumed(object, Assumption::PartnerLoginMemoKept);
    assert!(refused(b.run_in("Unm Driver", "Read")));
}
