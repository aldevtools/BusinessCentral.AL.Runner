//! `Codeunit.Run(Id, V)` of a codeunit with TableNo, where the Variant V holds a RecordRef. It compiles (a RecordRef
//! itself does not: must-not-compile 1786/1787), and Microsoft's Batch Processing Mgt. (codeunit 1380, InvokeProcessing)
//! relies on it. Rec is the RecordRef's row with its filters, a Modify stores the change and the RecordRef reads it after
//! the run; a RecordRef of another table fails before the run, also in the guarded form
//! (recordref/lt-recordref-variant-record/RecordRefVariantToRecord.al, QP_CodeunitRun_* tests).
//! Before, the runtime ran OnRun on a blank Rec (Tests-ERM 139444 "Unhandled UI: Page 700").

use al_builtins::native::Signal;
use al_testkit::{run, text};

const SRC: &str = r#"
table 50100 "Rep T"
{
    fields { field(1; K; Integer) { } field(2; D; Text[30]) { } }
    keys { key(PK; K) { } }
}
table 50103 "Rep Other"
{
    fields { field(1; K; Integer) { } }
    keys { key(PK; K) { } }
}
codeunit 50104 "Rep Log"
{
    SingleInstance = true;
    var
        Entries: Text;
    procedure Add(E: Text)
    begin
        Entries += E + ';';
    end;
    procedure Take(): Text
    var
        R: Text;
    begin
        R := Entries;
        Entries := '';
        exit(R);
    end;
}
codeunit 50101 "Rep Run"
{
    TableNo = "Rep T";
    trigger OnRun()
    var
        Log: Codeunit "Rep Log";
    begin
        Log.Add('onrun k=' + Format(Rec.K) + ' d=' + Rec.D + ' filters=[' + Rec.GetFilters() + ']');
        Rec.D := 'changed';
        Rec.Modify();
    end;
}
codeunit 50102 "Rep Caller"
{
    local procedure Seed()
    var
        T: Record "Rep T";
        O: Record "Rep Other";
    begin
        T.DeleteAll();
        T.K := 1;
        T.D := 'first';
        T.Insert();
        T.K := 2;
        T.D := 'second';
        T.Insert();
        O.DeleteAll();
        O.K := 1;
        O.Insert();
    end;

    procedure Plain(): Text
    var
        T: Record "Rep T";
        Log: Codeunit "Rep Log";
        RecRef: RecordRef;
        V: Variant;
    begin
        Seed();
        RecRef.Open(Database::"Rep T");
        RecRef.Field(1).SetRange(2);
        RecRef.FindFirst();
        RecRef.Field(1).SetRange();
        V := RecRef;
        Codeunit.Run(Codeunit::"Rep Run", V);
        T.Get(2);
        exit(Log.Take() + Format(RecRef.Field(2).Value) + '|' + T.D);
    end;

    procedure Filtered(): Text
    var
        Log: Codeunit "Rep Log";
        RecRef: RecordRef;
        V: Variant;
    begin
        Seed();
        RecRef.Open(Database::"Rep T");
        RecRef.Field(2).SetFilter('first|second');
        RecRef.FindLast();
        V := RecRef;
        Codeunit.Run(Codeunit::"Rep Run", V);
        exit(Log.Take() + Format(RecRef.Field(2).Value) + '|' + RecRef.GetFilters());
    end;

    procedure OtherTable(): Text
    var
        Log: Codeunit "Rep Log";
        RecRef: RecordRef;
        V: Variant;
    begin
        Seed();
        Commit();
        RecRef.Open(Database::"Rep Other");
        RecRef.FindFirst();
        V := RecRef;
        asserterror if Codeunit.Run(Codeunit::"Rep Run", V) then;
        exit(GetLastErrorText() + '|' + Log.Take());
    end;
}
"#;

/// RecordRefVariantToRecord.al QP_CodeunitRun_VariantRecordRef_Plain.
#[test]
fn rec_is_the_recordrefs_row_and_the_change_reaches_it() {
    assert_eq!(text(run(SRC, "Plain")), "onrun k=2 d=second filters=[];changed|changed");
}

/// RecordRefVariantToRecord.al QP_CodeunitRun_VariantRecordRef_Filtered.
#[test]
fn the_recordrefs_filters_carry_over() {
    assert_eq!(
        text(run(SRC, "Filtered")),
        "onrun k=2 d=second filters=[D: first|second];changed|D: first|second"
    );
}

/// RecordRefVariantToRecord.al QP_CodeunitRun_VariantRecordRef_OtherTable: the guarded form does not catch it.
#[test]
fn a_recordref_of_another_table_fails_before_the_run() {
    assert_eq!(
        text(run(SRC, "OtherTable")),
        "Record(50103) is not compatible with Codeunit.Run(Record(50100)).|"
    );
}

/// A Variant that holds an unopened RecordRef: not pinned.
#[test]
fn an_unopened_recordref_is_refused() {
    let src = SRC.replace("RecRef.Open(Database::\"Rep T\");\n        RecRef.Field(1).SetRange(2);\n        RecRef.FindFirst();\n        RecRef.Field(1).SetRange();\n", "");
    let r = run(&src, "Plain");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}
