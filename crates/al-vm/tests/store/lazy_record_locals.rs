//! Record locals built on first use (PERF-28 stage 1, locals only). A declared local of a stored table starts unbuilt and is
//! built by the statement that first names it; the session counts both (`Session::lazy_record_stats`, per session, so the
//! exact numbers hold with the tests running in parallel).
//!
//! Three kinds of test:
//! - `deferral_*`: the counts the feature gives. They are red until `Session::defers_local` and `build_local` are filled in.
//! - `eager_*`: the locals and the sessions that must stay built at frame entry (`deferred` stays 0).
//! - `same_*`: what AL code sees from an untouched local. These hold before and after, and pin that deferral changes nothing
//!   observable. Each expectation is the value the runner gave before deferral existed.

use al_builtins::value::Value;
use al_testkit::{Built, text};
use al_vm::{CaptureConfig, DebugHook, DebugView, LazyRecordStats, Resume};

const SRC: &str = r#"
table 50900 "LZ Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Payload; Integer) { InitValue = 7; }
        field(3; Note; Text[50]) { InitValue = 'init'; }
        field(4; Data; Blob) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 50901 "LZ Temp"
{
    TableType = Temporary;
    fields { field(1; "No."; Code[20]) { } field(2; Payload; Integer) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}

page 50902 "LZ Card"
{
    PageType = Card;
    SourceTable = "LZ Row";
    UsageCategory = None;
    layout { area(Content) { field(NoCtl; Rec."No.") { } field(PayloadCtl; Rec.Payload) { } } }
}

codeunit 50903 C
{
    // ---- deferral ----
    procedure Untouched()
    var
        A: Record "LZ Row";
    begin
    end;

    procedure BranchNotTaken(Take: Boolean)
    var
        A: Record "LZ Row";
    begin
        if Take then
            A.Payload := 1;
    end;

    procedure BranchTaken()
    var
        A: Record "LZ Row";
        Take: Boolean;
    begin
        Take := true;
        if Take then
            A.Payload := 1;
    end;

    procedure InLoop(): Integer
    var
        A: Record "LZ Row";
        I: Integer;
    begin
        for I := 1 to 3 do
            A.Payload += 1;
        exit(A.Payload);
    end;

    procedure GetAndUnused()
    var
        A: Record "LZ Row";
        B: Record "LZ Row";
    begin
        if A.Get('X') then;
    end;

    procedure NamedTwice(): Integer
    var
        A: Record "LZ Row";
    begin
        A.Payload := 1;
        A.Payload := A.Payload + 1;
        exit(A.Payload);
    end;

    // ---- eager ----
    procedure TemporaryLocal()
    var
        A: Record "LZ Row" temporary;
    begin
    end;

    procedure ArrayLocal()
    var
        A: array[2] of Record "LZ Row";
    begin
    end;

    procedure NamedReturn() R: Record "LZ Row"
    begin
    end;

    procedure UnpassedParameter(P: Record "LZ Row")
    begin
    end;

    procedure TemporaryTableLocal()
    var
        A: Record "LZ Temp";
    begin
    end;

    // ---- same ----
    local procedure SetPayload(var R: Record "LZ Row")
    begin
        R.Payload := 42;
    end;

    procedure ByVar(): Text
    var
        A: Record "LZ Row";
    begin
        SetPayload(A);
        exit(Format(A.Payload));
    end;

    local procedure Made(): Record "LZ Row"
    var
        A: Record "LZ Row";
    begin
        exit(A);
    end;

    procedure ExitRec(): Text
    var
        B: Record "LZ Row";
    begin
        B := Made();
        exit(Format(B.Payload) + '|' + B.Note + '|' + Format(B.IsEmpty()));
    end;

    procedure ClearAndReset(): Text
    var
        A: Record "LZ Row";
        B: Record "LZ Row";
        C: Record "LZ Row";
    begin
        A.SetFilter("No.", 'X');
        A.Reset();
        B.Payload := 9;
        Clear(B);
        exit(Format(A.GetFilters()) + '|' + Format(B.Payload) + '|' + Format(C.Payload));
    end;

    procedure Security(): Text
    var
        [SecurityFiltering(SecurityFilter::Filtered)]
        A: Record "LZ Row";
        B: Record "LZ Row";
    begin
        exit(Format(A.SecurityFiltering()) + '|' + Format(B.SecurityFiltering()));
    end;

    procedure InitValues(): Text
    var
        A: Record "LZ Row";
    begin
        exit(Format(A.Payload) + '|' + A.Note + '|' + A."No." + '|' + Format(A.Data.HasValue()));
    end;

    local procedure W(var R: Record "LZ Row"; Txt: Text)
    var
        OS: OutStream;
    begin
        R.Data.CreateOutStream(OS);
        OS.WriteText(Txt);
    end;

    local procedure ByValue(R: Record "LZ Row")
    var
        OS: OutStream;
    begin
        R.Data.CreateOutStream(OS);
        OS.WriteText('zz');
    end;

    local procedure Rd(var R: Record "LZ Row"): Text
    var
        IS: InStream;
        T: Text;
    begin
        R.CalcFields(Data);
        R.Data.CreateInStream(IS);
        IS.ReadText(T);
        exit(Format(R.Data.HasValue()) + ':' + T);
    end;

    // A Blob local born before a Commit, first named after it (blob_value_copy_shapes.rs, NeighborCommitWhileFreshThenInsert).
    procedure BlobAfterCommit(): Text
    var
        Src: Record "LZ Row";
    begin
        Commit();
        Src."No." := 'B1';
        W(Src, 'alpha');
        Src.Insert();
        ByValue(Src);
        exit(Rd(Src));
    end;

    // The same with a second Commit between the Insert and the by-value call.
    procedure BlobAfterTwoCommits(): Text
    var
        Src: Record "LZ Row";
    begin
        Commit();
        Src."No." := 'B2';
        W(Src, 'alpha');
        Src.Insert();
        Commit();
        ByValue(Src);
        exit(Rd(Src));
    end;

    [TryFunction]
    local procedure Fails(var N: Integer)
    var
        A: Record "LZ Row";
        B: Record "LZ Row";
    begin
        A.Payload := 5;
        N := 5;
        Error('boom');
    end;

    procedure TryFinals(): Text
    var
        N: Integer;
        Ok: Boolean;
        Untouched: Record "LZ Row";
    begin
        Ok := Fails(N);
        exit(Format(Ok) + '|' + Format(N) + '|' + GetLastErrorText());
    end;

    procedure TrapPage(): Text
    var
        Row: Record "LZ Row";
        Other: Record "LZ Row";
        Card: TestPage "LZ Card";
    begin
        Row."No." := 'P1';
        Row.Payload := 11;
        Row.Insert();
        Card.Trap();
        Page.Run(Page::"LZ Card", Row);
        exit(Card.NoCtl.Value() + '|' + Card.PayloadCtl.Value() + '|' + Format(Other.Payload));
    end;
}
"#;

/// Runs `routine` of codeunit C on a fresh session; the result and the session's counts.
fn run(routine: &str) -> (Result<Value, al_builtins::native::Signal>, LazyRecordStats) {
    let mut b = Built::from_source(SRC);
    let r = b.run_in("C", routine);
    (r, b.session.lazy_record_stats())
}

fn counts(routine: &str) -> LazyRecordStats {
    let (r, stats) = run(routine);
    r.unwrap_or_else(|e| panic!("{routine}: {e:?}"));
    stats
}

fn stats(deferred: u64, built: u64) -> LazyRecordStats {
    LazyRecordStats { deferred, built }
}

fn same(routine: &str) -> String {
    text(run(routine).0)
}

// ---- deferral: red until the feature is on ----

#[test]
fn deferral_an_untouched_local_is_never_built() {
    assert_eq!(counts("Untouched"), stats(1, 0));
}

#[test]
fn deferral_a_local_named_in_a_branch_not_taken_is_never_built() {
    let mut b = Built::from_source(SRC);
    let (oi, ri) = b.routine_in("C", "BranchNotTaken");
    b.call(oi, ri, vec![Value::Boolean(false)]).expect("runs");
    assert_eq!(b.session.lazy_record_stats(), stats(1, 0));
}

#[test]
fn deferral_a_local_named_in_a_branch_taken_is_built_once() {
    assert_eq!(counts("BranchTaken"), stats(1, 1));
}

#[test]
fn deferral_a_local_named_in_a_loop_body_is_built_once() {
    let (r, s) = run("InLoop");
    assert_eq!(al_testkit::int(r), 10);
    assert_eq!(s, stats(1, 1));
}

#[test]
fn deferral_a_get_builds_its_local_and_an_unnamed_local_stays_unbuilt() {
    assert_eq!(counts("GetAndUnused"), stats(2, 1));
}

#[test]
fn deferral_a_local_named_twice_at_the_top_level_is_built_once() {
    let (r, s) = run("NamedTwice");
    assert_eq!(al_testkit::int(r), 2);
    assert_eq!(s, stats(1, 1));
}

/// Emission is final in A0, so this half is green already: later statements of the routine's own block do not name the
/// local to the build again.
#[test]
fn deferral_only_the_first_top_level_statement_that_names_a_local_builds_it() {
    let b = Built::from_source(SRC);
    let (oi, ri) = b.routine_in("C", "NamedTwice");
    let ops = al_vm::exec_disassemble(&b.lowered.program, oi, ri)
        .expect("compiles")
        .join("\n");
    assert_eq!(ops.matches("BuildLocal").count(), 1, "{ops}");
}

// ---- eager: stays built at frame entry ----

#[test]
fn eager_a_temporary_local() {
    assert_eq!(counts("TemporaryLocal"), stats(0, 0));
}

#[test]
fn eager_an_array_of_records() {
    assert_eq!(counts("ArrayLocal"), stats(0, 0));
}

#[test]
fn eager_the_named_return_value() {
    assert_eq!(counts("NamedReturn"), stats(0, 0));
}

#[test]
fn eager_a_parameter_that_is_not_passed() {
    assert_eq!(counts("UnpassedParameter"), stats(0, 0));
}

#[test]
fn eager_a_local_of_a_temporary_table() {
    assert_eq!(counts("TemporaryTableLocal"), stats(0, 0));
}

struct Resumes;

impl DebugHook for Resumes {
    fn statement(&mut self, _view: &DebugView<'_>) -> Resume {
        Resume::Continue
    }
}

#[test]
fn eager_with_a_debugger_attached() {
    let mut b = Built::from_source(SRC);
    b.session.attach_debugger(Box::new(Resumes));
    b.run_in("C", "Untouched").expect("runs");
    assert_eq!(b.session.lazy_record_stats(), stats(0, 0));
}

#[test]
fn eager_with_value_capture_on() {
    let mut b = Built::from_source(SRC);
    b.session.enable_capture(CaptureConfig {
        values: true,
        loops: false,
    });
    b.run_in("C", "Untouched").expect("runs");
    assert_eq!(b.session.lazy_record_stats(), stats(0, 0));
}

// ---- same: what AL code sees is what it saw before ----

#[test]
fn same_a_local_passed_by_var_takes_the_callees_write() {
    assert_eq!(same("ByVar"), "42");
}

#[test]
fn same_a_record_exited_from_a_routine() {
    assert_eq!(same("ExitRec"), "7|init|Yes");
}

#[test]
fn same_clear_and_reset_of_untouched_and_used_locals() {
    assert_eq!(same("ClearAndReset"), "|7|7");
}

#[test]
fn same_security_filtering_reads_back() {
    assert_eq!(same("Security"), "Filtered|Validated");
}

#[test]
fn same_init_values_of_an_untouched_local() {
    assert_eq!(same("InitValues"), "7|init||No");
}

#[test]
fn same_a_blob_local_first_named_after_a_commit() {
    assert_eq!(same("BlobAfterCommit"), "Yes:zzpha");
    assert_eq!(same("BlobAfterTwoCommits"), "Yes:zzpha");
}

#[test]
fn same_a_try_function_keeps_its_by_var_finals_with_unbuilt_locals() {
    assert_eq!(same("TryFinals"), "No|5|boom");
}

#[test]
fn same_a_test_page_trap_in_a_routine_with_record_locals() {
    assert_eq!(same("TrapPage"), "P1|11|7");
}
