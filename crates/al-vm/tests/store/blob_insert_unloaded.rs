//! An `Insert` of a record whose Blob holds stored content that was never loaded: a record read with `Get` and no
//! `CalcFields`, or a copy of one (`:=`, `Record.Copy`), stored under a new key (issue 9286), in a database or a temporary
//! table. No corpus case pins what the new row's Blob holds then, so the Insert is refused. A loaded Blob (`CalcFields`
//! before the copy) and a stored empty Blob insert as before.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

fn assert_refused(r: Result<al_builtins::value::Value, Signal>) {
    match r {
        Err(Signal::Unsupported(reason)) => assert!(reason.contains("not loaded"), "{reason}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

const SRC: &str = r#"
table 50130 "Bi Row"
{
    fields { field(1; K; Code[20]) { } field(2; Body; Blob) { } }
    keys { key(PK; K) { } }
}
codeunit 50131 C
{
    local procedure Seed(K: Code[20]; T: Text)
    var R: Record "Bi Row"; OutS: OutStream;
    begin
        R.K := K;
        if T <> '' then begin
            R.Body.CreateOutStream(OutS);
            OutS.WriteText(T);
        end;
        R.Insert();
    end;

    local procedure Stored(K: Code[20]): Text
    var R: Record "Bi Row"; InS: InStream; T: Text;
    begin
        R.Get(K);
        R.CalcFields(Body);
        if not R.Body.HasValue() then exit('nohv:');
        R.Body.CreateInStream(InS);
        InS.ReadText(T);
        exit('hv:' + T);
    end;

    procedure AssignNoCalc(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('A', 'hello');
        L.Get('A');
        C := L;
        C.K := 'A2';
        C.Insert();
        exit(Stored('A2'));
    end;

    procedure SameVariableNoCalc(): Text
    var L: Record "Bi Row";
    begin
        Seed('B', 'hello');
        L.Get('B');
        L.K := 'B2';
        L.Insert();
        exit(Stored('B2'));
    end;

    procedure CopyNoCalc(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('D', 'hello');
        L.Get('D');
        C.Copy(L);
        C.K := 'D2';
        C.Insert();
        exit(Stored('D2'));
    end;

    procedure TemporaryNoCalc(): Text
    var T: Record "Bi Row" temporary; OutS: OutStream;
    begin
        T.K := 'G';
        T.Body.CreateOutStream(OutS);
        OutS.WriteText('hello');
        T.Insert();
        T.Get('G');
        T.K := 'G2';
        T.Insert();
        exit('inserted');
    end;

    procedure AssignWithCalc(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('E', 'hello');
        L.Get('E');
        L.CalcFields(Body);
        C := L;
        C.K := 'E2';
        C.Insert();
        exit(Stored('E2'));
    end;

    procedure AssignNoCalcStoredEmpty(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('F', '');
        L.Get('F');
        C := L;
        C.K := 'F2';
        C.Insert();
        exit(Stored('F2'));
    end;
}
"#;

#[test]
fn an_insert_of_an_assigned_copy_of_a_blob_not_loaded_is_refused() {
    assert_refused(run_in(SRC, "C", "AssignNoCalc"));
}

#[test]
fn an_insert_under_a_new_key_of_a_record_whose_blob_is_not_loaded_is_refused() {
    assert_refused(run_in(SRC, "C", "SameVariableNoCalc"));
}

#[test]
fn an_insert_of_a_record_copy_of_a_blob_not_loaded_is_refused() {
    assert_refused(run_in(SRC, "C", "CopyNoCalc"));
}

#[test]
fn an_insert_under_a_new_key_of_a_temporary_row_read_without_calcfields_is_refused() {
    assert_refused(run_in(SRC, "C", "TemporaryNoCalc"));
}

#[test]
fn an_insert_of_an_assigned_copy_of_a_loaded_blob_stores_its_content() {
    assert_eq!(text(run_in(SRC, "C", "AssignWithCalc")), "hv:hello");
}

#[test]
fn an_insert_of_an_assigned_copy_of_a_stored_empty_blob_stores_none() {
    assert_eq!(text(run_in(SRC, "C", "AssignNoCalcStoredEmpty")), "nohv:");
}
