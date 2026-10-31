//! A Delete(false) through a stale buffer from the variable's own Insert, made stale by another variable's Modify and
//! Commit, on a table with an OnBeforeDeleteEvent subscriber: the subscriber runs, then the Delete raises DB:RecordChanged
//! and the row stays (al-runner issue 6546, the reporter's repro passed on real BC; corpus writer Y32 pins it). On the
//! awaiting assumption `stale-delete-event-before-record-changed` until that file is on the corpus master.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 50381 "Run Entry"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Ending Time"; Time) { }
    }
    keys { key(PK; "Entry No.") { } }
    trigger OnDelete()
    begin
        DeleteLog.Add('OnDelete;');
    end;
    var DeleteLog: Codeunit "Delete Log";
}
codeunit 50381 "Delete Log"
{
    SingleInstance = true;
    var Log: Text;

    procedure Add(Entry: Text)
    begin
        Log += Entry;
    end;

    procedure Take(): Text
    var Result: Text;
    begin
        Result := Log;
        Log := '';
        exit(Result);
    end;

    [EventSubscriber(ObjectType::Table, Database::"Run Entry", 'OnBeforeDeleteEvent', '', false, false)]
    local procedure LogBeforeDelete(var Rec: Record "Run Entry"; RunTrigger: Boolean)
    begin
        Add('OnBeforeDeleteEvent;');
    end;
}
codeunit 50391 C
{
    var DeleteLog: Codeunit "Delete Log";

    local procedure MakeStaleBuffer(var RunEntry: Record "Run Entry")
    var Other: Record "Run Entry";
    begin
        Other.DeleteAll();
        RunEntry."Entry No." := 1;
        RunEntry.Insert();
        Commit();
        Other.Get(1);
        Other."Ending Time" := 170000T;
        Other.Modify();
        Commit();
        DeleteLog.Take();
    end;

    procedure StaleDelete(): Text
    var RunEntry: Record "Run Entry";
    begin
        MakeStaleBuffer(RunEntry);
        asserterror RunEntry.Delete(false);
        exit(GetLastErrorCode() + ' / ' + DeleteLog.Take() + ' / rows ' + Format(RunEntry.Count()));
    end;

    procedure StaleDeleteTrue(): Text
    var RunEntry: Record "Run Entry";
    begin
        MakeStaleBuffer(RunEntry);
        asserterror RunEntry.Delete(true);
        exit(GetLastErrorCode() + ' / ' + DeleteLog.Take());
    end;
}
"#;

fn run(proc: &str) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_in(SRC, "C", proc)
}

/// The reporter's repro (al-runner issue 6546): StaleDeleteRaisesRecordChanged and StaleDeleteRunsTheEventBeforeTheCheck.
#[test]
fn the_before_delete_event_runs_and_then_the_delete_raises_record_changed() {
    clear_hits();
    assert_eq!(
        text(run("StaleDelete")),
        "DB:RecordChanged / OnBeforeDeleteEvent; / rows 1"
    );
    assert!(take_hits().contains(&Assumption::StaleDeleteEventBeforeRecordChanged));
}

/// Delete(true): where the OnDelete trigger falls against the check is not pinned yet.
#[test]
fn a_stale_delete_with_the_delete_trigger_stays_refused() {
    match run("StaleDeleteTrue") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("not pinned"), "{m}"),
        other => panic!("{other:?}"),
    }
}
