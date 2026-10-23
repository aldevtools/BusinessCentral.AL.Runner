//! A Time or DateTime primary key value with milliseconds the SQL column rounds (al-runner issue 5689): corpus
//! errors/lt-ms-refusals-130/P130DateFilters.al:527-735, measured on BC 27.5, 28.5 and 29.0.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69960 "TK Time Key"
{
    fields
    {
        field(1; "Entry Code"; Code[20]) { }
        field(2; "Entry Time"; Time) { }
        field(3; Note; Text[30]) { }
    }
    keys { key(PK; "Entry Code", "Entry Time") { Clustered = true; } }
}
table 69961 "TK DateTime Key"
{
    fields
    {
        field(1; "At"; DateTime) { }
        field(2; Note; Text[30]) { }
    }
    keys { key(PK; "At") { Clustered = true; } }
}
codeunit 69962 TK
{
    local procedure KeyTime(var K: Record "TK Time Key"): Text
    begin
        exit(K."Entry Code" + '|' + Format(K."Entry Time", 0, 9) + '|' + Format(K."Entry Time" - 000000T) + '|' + K.Note);
    end;

    local procedure GetTime(Code: Code[20]; T: Time): Text
    var K: Record "TK Time Key";
    begin
        if K.Get(Code, T) then
            exit('found|' + KeyTime(K) + ';');
        exit('not found;');
    end;

    // P130DateFilters.al:527-623.
    procedure InsertGet(): Text
    var K: Record "TK Time Key"; s: Text;
    begin
        K.Init(); K."Entry Code" := 'A'; K."Entry Time" := 100000T + 1; K.Note := 'first';
        s := Format(K.Insert(true)) + ';' + KeyTime(K) + ';';
        s += GetTime('A', 100000T + 1) + GetTime('A', 100000T) + GetTime('A', 100000T + 2);
        K.Reset();
        K.FindFirst();
        s += KeyTime(K) + ';' + Format(K.Count()) + ';';
        K.SetRange("Entry Time", 100000T + 1);
        s += Format(K.Count()) + ';';
        K.Reset();
        K.Init(); K."Entry Code" := 'A'; K."Entry Time" := 100000T; K.Note := 'second';
        s += Format(K.Insert()) + ';';
        K.Init(); K."Entry Code" := 'B'; K."Entry Time" := 235959.999T; K.Note := 'late';
        s += Format(K.Insert()) + ';' + GetTime('B', 235959.999T) + GetTime('B', 000000T);
        exit(s);
    end;

    // P130DateFilters.al:568-589: a plain Insert of a duplicate key fails at the next read of the table.
    procedure PlainDuplicate(): Text
    var K: Record "TK Time Key"; Stage: Text; Cnt: Integer;
    begin
        K.Init(); K."Entry Code" := 'A'; K."Entry Time" := 100000T + 1; K.Insert();
        asserterror begin
            Stage := 'insert';
            K.Init(); K."Entry Code" := 'A'; K."Entry Time" := 100000T; K.Note := 'third';
            K.Insert();
            Stage := 'count';
            K.Reset();
            Cnt := K.Count();
            Stage := 'done';
        end;
        exit(Stage + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    // P130DateFilters.al:625-660.
    procedure ModifyRenameDelete(): Text
    var K: Record "TK Time Key"; s: Text;
    begin
        K.Init(); K."Entry Code" := 'A'; K."Entry Time" := 100000T + 1; K.Note := 'first';
        K.Insert();
        K.Note := 'modified';
        s := Format(K.Modify()) + ';' + GetTime('A', 100000T);
        K.Get('A', 100000T + 1);
        s += Format(K.Rename('A', 120000T + 1)) + ';' + KeyTime(K) + ';';
        s += GetTime('A', 120000T) + GetTime('A', 100000T);
        K.Init(); K."Entry Code" := 'A'; K."Entry Time" := 120000T + 1;
        s += Format(K.Delete()) + ';';
        K.Reset();
        exit(s + Format(K.Count()));
    end;

    // P130DateFilters.al:662-735.
    procedure DateTimeKey(): Text
    var K: Record "TK DateTime Key"; Base: DateTime; s: Text;
    begin
        Base := CreateDateTime(20240325D, 100000T);
        K.Init(); K."At" := Base + 1; K.Note := 'first';
        s := Format(K.Insert()) + ';' + Format(K."At", 0, 9) + ';';
        Clear(K);
        s += Format(K.Get(Base + 1)) + '|' + Format(K."At", 0, 9) + ';';
        Clear(K);
        s += Format(K.Get(Base)) + '|' + Format(K."At", 0, 9) + ';';
        K.Reset();
        K.FindFirst();
        s += Format(K."At", 0, 9) + '|' + Format(K."At" - Base) + ';';
        K.Init(); K."At" := Base; K.Note := 'second';
        s += Format(K.Insert()) + ';';
        K.DeleteAll();
        K.Init(); K."At" := CreateDateTime(20240325D, 235959.999T); K.Note := 'late';
        s += Format(K.Insert()) + ';';
        Clear(K);
        s += Format(K.Get(CreateDateTime(20240326D, 000000T))) + '|' + Format(K."At", 0, 9);
        exit(s);
    end;

    // Not pinned: the error of a Get without its result used, by a value the column rounds, that finds no row.
    procedure GetBareRoundedMissing()
    var K: Record "TK Time Key";
    begin
        K.Get('A', 100000T + 2);
    end;

    // Not pinned: an Insert whose key only rounds onto an existing row.
    procedure InsertRoundsOntoExisting(): Boolean
    var K: Record "TK Time Key";
    begin
        K.Init(); K."Entry Code" := 'A'; K."Entry Time" := 100000T; K.Insert();
        K.Init(); K."Entry Code" := 'A'; K."Entry Time" := 100000T + 1;
        exit(K.Insert());
    end;

    // Not pinned: a Rename whose new key only rounds onto an existing row.
    procedure RenameRoundsOntoExisting()
    var K: Record "TK Time Key";
    begin
        K.Init(); K."Entry Code" := 'A'; K."Entry Time" := 100000T; K.Insert();
        K.Init(); K."Entry Code" := 'B'; K."Entry Time" := 100000T; K.Insert();
        K.Rename('A', 100000T + 1);
    end;
}
"#;

fn run(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_in(SRC, "TK", routine)
}

// P130DateFilters.al:603-623.
#[test]
fn a_time_key_with_milliseconds_is_stored_rounded_and_found_by_get() {
    assert_eq!(
        text(run("InsertGet")),
        "Yes;A|10:00:00.001|36000001|first;\
         found|A|10:00:00|36000000|first;found|A|10:00:00|36000000|first;not found;\
         A|10:00:00|36000000|first;1;1;No;\
         Yes;found|B|00:00:00|0|late;found|B|00:00:00|0|late;"
    );
}

// P130DateFilters.al:610-613.
#[test]
fn a_plain_duplicate_insert_fails_at_the_next_read() {
    assert_eq!(
        text(run("PlainDuplicate")),
        "count|The record in table TK Time Key already exists. Identification fields and values: Entry Code='A',Entry Time='10:00:00 AM'|DB:RecordExists"
    );
}

// P130DateFilters.al:651-658.
#[test]
fn modify_rename_and_delete_work_through_the_unrounded_key() {
    assert_eq!(
        text(run("ModifyRenameDelete")),
        "Yes;found|A|10:00:00|36000000|modified;Yes;A|12:00:00.001|43200001|modified;\
         found|A|12:00:00|43200000|modified;not found;Yes;0"
    );
}

// P130DateFilters.al:720-733.
#[test]
fn a_datetime_key_rounds_and_rolls_over_to_the_next_day() {
    assert_eq!(
        text(run("DateTimeKey")),
        "Yes;2024-03-25T10:00:00.001Z;Yes|2024-03-25T10:00:00Z;Yes|2024-03-25T10:00:00Z;\
         2024-03-25T10:00:00Z|;No;Yes;Yes|2024-03-26T00:00:00Z"
    );
}

#[test]
fn unpinned_rounded_key_cases_are_refused() {
    for routine in [
        "GetBareRoundedMissing",
        "InsertRoundsOntoExisting",
        "RenameRoundsOntoExisting",
    ] {
        let r = run(routine);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
    }
}
