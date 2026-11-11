//! Recorded runs (ADR-073 decisions 1 to 6): a replay leaves what the real run leaves, and a session whose inputs differ
//! takes none.

use al_builtins::value::Value;
use al_vm::{Engine, Session};
use std::sync::Arc;

const SRC: &str = r#"
table 50900 "Rr Setup"
{
    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; Active; Boolean) { }
    }
    keys { key(PK; "Primary Key") { } }
}

codeunit 50901 "Rr Single"
{
    SingleInstance = true;

    var
        Setup: Record "Rr Setup";
        Day: Date;
        Hits: Integer;

    procedure Remember()
    begin
        Setup.Get();
        Day := WorkDate();
        Hits += 1;
    end;

    procedure Describe(): Text
    begin
        exit(Format(Setup.Active) + '|' + Format(Day, 0, 9) + '|' + Format(Hits));
    end;
}

codeunit 50902 "Rr Login"
{
    procedure Login()
    var
        Setup: Record "Rr Setup";
        Single: Codeunit "Rr Single";
    begin
        if not Setup.Get() then begin
            Setup.Init();
            Setup.Active := true;
            Setup.Insert();
        end;
        WorkDate(DMY2Date(17, 3, 2025));
        Single.Remember();
    end;

    procedure Probe(): Text
    var
        Setup: Record "Rr Setup";
        Single: Codeunit "Rr Single";
    begin
        exit(Format(Setup.Count()) + '|' + Format(WorkDate(), 0, 9) + '|' + Single.Describe());
    end;
}
"#;

fn routine(engine: &Engine, object: &str, name: &str) -> (usize, usize) {
    let o = engine
        .program
        .objects
        .iter()
        .position(|x| x.name.eq_ignore_ascii_case(object))
        .unwrap();
    let r = engine.program.objects[o]
        .routines
        .iter()
        .position(|x| x.name.eq_ignore_ascii_case(name))
        .unwrap();
    (o, r)
}

fn call(s: &mut Session, object: &str, name: &str) -> String {
    let (o, r) = routine(&s.engine.clone(), object, name);
    let inst = s.codeunit_instance(o);
    match s.call(o, r, inst, Vec::new()).unwrap().0 {
        Value::Text(t) => t.to_string(),
        _ => String::new(),
    }
}

fn engine() -> Arc<Engine> {
    al_testkit::engine(&al_testkit::compile(SRC))
}

/// The login run once for real and recorded.
fn recorded(engine: &Arc<Engine>) -> al_vm::RecordedRun {
    let mut s = Session::new(engine.clone());
    let r = s.start_recording().expect("a new session can record");
    call(&mut s, "Rr Login", "Login");
    s.commit_transaction();
    s.finish_recording(r)
        .expect("the login leaves only state the image holds")
}

#[test]
fn a_replay_leaves_the_rows_the_single_instance_globals_and_the_work_date_the_real_run_leaves() {
    let engine = engine();
    let run = recorded(&engine);
    assert!(run.wrote_rows());
    let mut real = Session::new(engine.clone());
    call(&mut real, "Rr Login", "Login");
    real.commit_transaction();
    let mut replayed = Session::new(engine.clone());
    assert!(replayed.replay_applies(&run));
    replayed.replay(&run).unwrap();
    let want = call(&mut real, "Rr Login", "Probe");
    assert_eq!(call(&mut replayed, "Rr Login", "Probe"), want);
    assert_eq!(want, "1|2025-03-17|Yes|2025-03-17|1");
    // The SingleInstance codeunit goes on from what the run left, in both.
    call(&mut real, "Rr Login", "Login");
    call(&mut replayed, "Rr Login", "Login");
    assert_eq!(
        call(&mut replayed, "Rr Login", "Probe"),
        call(&mut real, "Rr Login", "Probe")
    );
}

#[test]
fn a_session_whose_image_differs_takes_no_replay() {
    let engine = engine();
    let run = recorded(&engine);
    // Another WorkDate: the run would read another date.
    let mut s = Session::new(engine.clone());
    let day = s.work_date();
    s.set_work_date(day - 1);
    assert!(!s.replay_applies(&run));
    // A SingleInstance codeunit exists already.
    let mut s = Session::new(engine.clone());
    call(&mut s, "Rr Login", "Probe");
    assert!(!s.replay_applies(&run));
}

#[test]
fn a_session_that_notes_the_code_it_runs_neither_records_nor_takes_a_replay() {
    let engine = engine();
    let run = recorded(&engine);
    let mut s = Session::new(engine.clone());
    s.enable_impact_trace();
    assert!(!s.replay_applies(&run));
    assert_eq!(s.start_recording().err(), Some(al_vm::NotRecorded::NotesCode));
}

#[test]
fn a_run_that_left_a_temporary_record_in_a_single_instance_global_is_not_recorded() {
    let src = r#"
table 50910 "Rt Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50911 "Rt Single"
{
    SingleInstance = true;
    var Buffer: Record "Rt Row" temporary;
    procedure Fill() begin Buffer.Id := 1; Buffer.Insert(); end;
}
codeunit 50912 "Rt Run"
{
    procedure Run() var Single: Codeunit "Rt Single"; begin Single.Fill(); end;
}
"#;
    let engine = al_testkit::engine(&al_testkit::compile(src));
    let mut s = Session::new(engine.clone());
    let r = s.start_recording().unwrap();
    call(&mut s, "Rt Run", "Run");
    s.commit_transaction();
    assert!(s.finish_recording(r).is_err());
}
