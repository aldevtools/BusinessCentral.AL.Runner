//! `IsEmpty` of G/L Entry against the corpus database's company, before the session's code wrote to it: BC holds
//! G/L entries there (test-framework/RlWorkDateAtStart.al:40 read, :48 `G/L entries exist` Yes). Only the unfiltered
//! `IsEmpty` of the session's own company is answered; every other shape keeps the demo-row refusal (records/op_inner.rs).

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 17 "G/L Entry"
{
    fields { field(1; "Entry No."; Integer) { } field(4; "Posting Date"; Date) { } }
    keys { key(PK; "Entry No.") { } }
}
table 79 "Company Information"
{
    fields { field(1; "Primary Key"; Code[10]) { } }
    keys { key(PK; "Primary Key") { } }
}
codeunit 69989 R
{
    procedure Plain(): Text
    var GLEntry: Record "G/L Entry";
    begin
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure Filtered(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry.SetRange("Entry No.", 1);
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure FilteredOtherGroup(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry.FilterGroup(2);
        GLEntry.SetRange("Entry No.", 1);
        GLEntry.FilterGroup(0);
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure MarkedOnly(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry.MarkedOnly(true);
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure Temporary(): Text
    var GLEntry: Record "G/L Entry" temporary;
    begin
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure OwnCompanyByName(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry.ChangeCompany(CompanyName());
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure Counted(): Text
    var GLEntry: Record "G/L Entry";
    begin
        exit(Format(GLEntry.Count()));
    end;
    procedure WriteThenIsEmpty(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry."Entry No." := 1;
        GLEntry.Insert();
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure DeleteAllEntries(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry.DeleteAll();
        exit('');
    end;
    // al-runner issue 9277: a write that asserterror rolls back leaves the table as it was.
    procedure RolledBackWriteThenIsEmpty(): Text
    var GLEntry: Record "G/L Entry";
    begin
        asserterror begin
            GLEntry."Entry No." := 5;
            GLEntry.Insert();
            Error('x');
        end;
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure RolledBackDeleteAllThenIsEmpty(): Text
    var GLEntry: Record "G/L Entry";
    begin
        asserterror begin
            GLEntry.DeleteAll();
            Error('x');
        end;
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure CommittedWriteThenRolledBackWriteThenIsEmpty(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry."Entry No." := 1;
        GLEntry.Insert();
        Commit();
        asserterror begin
            GLEntry."Entry No." := 5;
            GLEntry.Insert();
            Error('x');
        end;
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure RolledBackWriteThenCount(): Text
    var Info: Record "Company Information";
    begin
        asserterror begin
            Info."Primary Key" := 'X';
            Info.Insert();
            Error('x');
        end;
        exit(Format(Info.Count()));
    end;
    procedure CommittedWriteThenRolledBackWriteThenCount(): Text
    var Info: Record "Company Information";
    begin
        Info."Primary Key" := 'A';
        Info.Insert();
        Commit();
        asserterror begin
            Info."Primary Key" := 'X';
            Info.Insert();
            Error('x');
        end;
        exit(Format(Info.Count()));
    end;
    procedure InsertThenDeleteOne(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry."Entry No." := 1;
        GLEntry.Insert();
        GLEntry.Delete();
        exit('');
    end;
}
"#;

fn session(demo: bool) -> impl Fn(&str) -> Result<al_builtins::value::Value, Signal> {
    let l = al_testkit::compile(SRC);
    let mut engine = al_testkit::engine(&l);
    if demo {
        for name in ["G/L Entry", "Company Information"] {
            let t = l.program.objects.iter().position(|o| o.name == name).unwrap();
            std::sync::Arc::get_mut(&mut engine).unwrap().demo_tables.insert(t);
        }
    }
    let oi = l.program.objects.iter().position(|o| o.name == "R").unwrap();
    move |routine: &str| {
        let mut s = al_vm::Session::new(engine.clone());
        let ri = l.program.objects[oi]
            .routines
            .iter()
            .position(|r| r.name == routine)
            .unwrap();
        let inst = s.instance(oi);
        s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
    }
}

/// One session that runs `steps` in order: a routine name, `start` (the runner's `start_test_method`), `install` and
/// `end-install` (install scope on and off), `handover` (a new session restored from the baseline this one captures, as
/// each test codeunit after the shared install step starts) and `bytes` (the same through the baseline's bytes form, as
/// another thread reads it); the result of the last routine.
fn one_session(steps: &[&str]) -> Result<al_builtins::value::Value, Signal> {
    let l = al_testkit::compile(SRC);
    let mut engine = al_testkit::engine(&l);
    let t = l.program.objects.iter().position(|o| o.name == "G/L Entry").unwrap();
    std::sync::Arc::get_mut(&mut engine).unwrap().demo_tables.insert(t);
    let oi = l.program.objects.iter().position(|o| o.name == "R").unwrap();
    let mut s = al_vm::Session::new(engine.clone());
    let mut last = Ok(al_builtins::value::Value::Text(String::new().into()));
    for step in steps {
        match *step {
            "start" => s.start_test_method(),
            "install" => s.set_install_scope(true),
            "end-install" => s.set_install_scope(false),
            "handover" => {
                let b = s.capture_baseline();
                s = al_vm::Session::new(engine.clone());
                s.restore_baseline(&b);
            }
            "bytes" => {
                let bytes = s.capture_baseline().to_bytes().expect("plain values");
                s = al_vm::Session::new(engine.clone());
                s.restore_baseline(&al_vm::Baseline::from_bytes(&bytes).expect("own bytes"));
            }
            routine => {
                let ri = l.program.objects[oi]
                    .routines
                    .iter()
                    .position(|r| r.name == routine)
                    .unwrap();
                let inst = s.instance(oi);
                last = s.call(oi, ri, inst, vec![]).map(|(v, _)| v);
            }
        }
    }
    last
}

fn refused(r: Result<al_builtins::value::Value, Signal>) {
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("demo rows")),
        "{r:?}"
    );
}

// RlWorkDateAtStart.al:40/:48: the corpus company holds G/L entries before any test code ran.
#[test]
fn the_unfiltered_is_empty_of_gl_entry_answers_not_empty_in_the_corpus_company() {
    assert_eq!(text(session(true)("Plain")), "No");
}

// The pin reads an unfiltered record only: a filter, in any filter group, or MarkedOnly is not measured.
#[test]
fn a_filtered_or_marked_is_empty_of_gl_entry_keeps_the_refusal() {
    let call = session(true);
    refused(call("Filtered"));
    refused(call("FilteredOtherGroup"));
    refused(call("MarkedOnly"));
}

// The pin reads the session's company without ChangeCompany; a ChangeCompany record is not measured.
#[test]
fn an_is_empty_of_gl_entry_after_change_company_keeps_the_refusal() {
    refused(session(true)("OwnCompanyByName"));
}

// The pin measures IsEmpty only: Count keeps the refusal.
#[test]
fn a_count_of_gl_entry_keeps_the_refusal() {
    refused(session(true)("Counted"));
}

// A temporary record holds no database rows; after the session's own write the stored rows are its own: both stay as
// main answered them.
#[test]
fn a_temporary_record_or_one_after_a_write_answers_from_the_runtime_rows() {
    let call = session(true);
    assert_eq!(text(call("Temporary")), "Yes");
    assert_eq!(text(call("WriteThenIsEmpty")), "No");
}

// With --test-data or --company-data the runner fills no demo tables: IsEmpty answers from the runtime's own rows.
#[test]
fn without_demo_tables_is_empty_of_gl_entry_answers_from_the_runtime_rows() {
    assert_eq!(text(session(false)("Plain")), "Yes");
}

// A delete in an earlier test method of the same session stands in BC until the codeunit ends (default codeunit
// isolation): the demo rows may be gone, so a later test's IsEmpty keeps main's refusal.
#[test]
fn a_delete_all_in_an_earlier_test_method_keeps_the_refusal() {
    refused(one_session(&["start", "DeleteAllEntries", "start", "Plain"]));
}

#[test]
fn a_delete_of_one_row_in_an_earlier_test_method_keeps_the_refusal() {
    refused(one_session(&["start", "InsertThenDeleteOne", "start", "Plain"]));
}

// Install code that deleted G/L entries leaves BC's company without its demo rows: a test's IsEmpty keeps the refusal.
#[test]
fn a_delete_all_in_install_code_keeps_the_refusal() {
    refused(one_session(&[
        "install",
        "DeleteAllEntries",
        "end-install",
        "start",
        "Plain",
    ]));
}

// Install code on the shared install baseline (al-testrun `baseline.rs`) runs in a session of its own: the test codeunit's
// session starts from the baseline it left, so the mark goes with the rows.
#[test]
fn a_delete_all_in_install_code_handed_over_through_a_baseline_keeps_the_refusal() {
    refused(one_session(&[
        "install",
        "DeleteAllEntries",
        "end-install",
        "handover",
        "start",
        "Plain",
    ]));
}

// The same hand-over through the bytes form of the baseline (another thread's copy).
#[test]
fn a_delete_all_in_install_code_handed_over_through_baseline_bytes_keeps_the_refusal() {
    refused(one_session(&[
        "install",
        "DeleteAllEntries",
        "end-install",
        "bytes",
        "start",
        "Plain",
    ]));
}

// Control: a hand-over without a write keeps the answer.
#[test]
fn a_hand_over_without_a_write_keeps_the_answer() {
    assert_eq!(text(one_session(&["handover", "bytes", "start", "Plain"])), "No");
}

// The pin reads G/L Entry in a test method, outside install code: an IsEmpty inside install code keeps the refusal.
#[test]
fn an_is_empty_of_gl_entry_inside_install_code_keeps_the_refusal() {
    refused(one_session(&["install", "Plain"]));
}

// Control: two test methods that only read keep the answer.
#[test]
fn a_read_in_an_earlier_test_method_keeps_the_answer() {
    assert_eq!(text(one_session(&["start", "Plain", "start", "Plain"])), "No");
}

// Under runner 130450 a codeunit's OnRun runs in a session restored from the baseline the codeunit started from; what it
// left is handed on only when it changed something (al-testrun `numbers_taken.rs` `onrun_left`). A DeleteAll on the
// runtime's empty G/L Entry removes no row but sets the mark: the session no longer holds what it started from, and the
// table counts as changed, so the later codeunits start from what OnRun left and keep the refusal.
#[test]
fn an_onrun_delete_all_on_an_empty_store_counts_as_a_change() {
    let l = al_testkit::compile(SRC);
    let mut engine = al_testkit::engine(&l);
    let t = l.program.objects.iter().position(|o| o.name == "G/L Entry").unwrap();
    std::sync::Arc::get_mut(&mut engine).unwrap().demo_tables.insert(t);
    let oi = l.program.objects.iter().position(|o| o.name == "R").unwrap();
    let ri = |name: &str| {
        l.program.objects[oi]
            .routines
            .iter()
            .position(|r| r.name == name)
            .unwrap()
    };
    let mut s = al_vm::Session::new(engine.clone());
    let start = s.capture_baseline();
    s = al_vm::Session::new(engine.clone());
    s.restore_baseline(&start);
    let changes_from = s.store_mut().change_counter();
    let inst = s.instance(oi);
    s.call(oi, ri("DeleteAllEntries"), inst, vec![]).expect("DeleteAll");
    assert!(!s.holds_what_it_started_from(&start));
    assert!(s.store_mut().tables_changed_since(changes_from).contains(&(t as u32)));
    // The later codeunit starts from what OnRun left.
    let left = s.capture_baseline();
    s = al_vm::Session::new(engine.clone());
    s.restore_baseline(&left);
    s.start_test_method();
    let inst = s.instance(oi);
    refused(s.call(oi, ri("Plain"), inst, vec![]).map(|(v, _)| v));
}

// al-runner issue 9277. asserterror rolls back the writes since the last Commit (codeunits/TestScopeIsolationContracts.al:
// Test04AssertErrorLocalVarChangesNotAffectCaller :81-101; codeunits/run-onrun/QuCommitIsolation.al:28, X07): the table
// holds what it held before, so the pinned IsEmpty of the corpus company answers as it does with no write
// (RlWorkDateAtStart.al:40/:48), not from the runtime's rows.
#[test]
fn a_rolled_back_write_of_gl_entry_keeps_the_pinned_answer() {
    assert_eq!(text(session(true)("RolledBackWriteThenIsEmpty")), "No");
    assert_eq!(text(session(true)("RolledBackDeleteAllThenIsEmpty")), "No");
}

// A Count of a demo table after a rolled-back write is the shape with no write: the demo-row refusal.
#[test]
fn a_rolled_back_write_of_a_demo_table_keeps_the_refusal_of_count() {
    refused(session(true)("RolledBackWriteThenCount"));
}

// A committed write stands through a later rollback (QuCommitIsolation.al:28, X07): the reads answer from the runtime's rows, as after any own write.
#[test]
fn a_committed_write_stands_through_a_later_rollback() {
    assert_eq!(
        text(session(true)("CommittedWriteThenRolledBackWriteThenIsEmpty")),
        "No"
    );
    assert_eq!(text(session(true)("CommittedWriteThenRolledBackWriteThenCount")), "1");
}
