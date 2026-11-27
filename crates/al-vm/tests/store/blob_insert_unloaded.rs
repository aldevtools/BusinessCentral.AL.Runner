//! An `Insert` of a record whose Blob holds stored content it never loaded (a read without `CalcFields`, or a copy of such a
//! record), and a `Modify` of such a copy under the key of another stored row (issue 9286). The open corpus probe (pull request
//! 1631, record/lt-blob-field-assign/BlobInsertUnloaded.al) measured on BC 29: the Insert stores no Blob content and the
//! inserting record answers HasValue Yes and reads empty; the Modify leaves the other row's Blob as it was. Through the
//! assumption `blob-never-loaded-not-stored`. A loaded Blob (`CalcFields` before the copy or on it) and an empty source store
//! as before.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50130 "Bi Row"
{
    fields { field(1; K; Code[20]) { } field(2; Body; Blob) { } field(3; Note; Text[100]) { } }
    keys { key(PK; K) { } }
    trigger OnInsert()
    var Before: Boolean;
    begin
        if K <> 'TRG' then exit;
        Before := Body.HasValue();
        CalcFields(Body);
        Note := StrSubstNo('before=%1 after=%2', Before, Body.HasValue());
    end;
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

    local procedure Show(var R: Record "Bi Row"): Text
    var InS: InStream; T: Text;
    begin
        R.Body.CreateInStream(InS);
        if R.Body.HasValue() then InS.ReadText(T);
        exit(StrSubstNo('%1[%2]', R.Body.HasValue(), T));
    end;

    // The stored row: HasValue before CalcFields, then HasValue and the text after it.
    local procedure Stored(K: Code[20]): Text
    var R: Record "Bi Row"; Before: Boolean;
    begin
        R.Get(K);
        Before := R.Body.HasValue();
        R.CalcFields(Body);
        exit(StrSubstNo('%1 %2', Before, Show(R)));
    end;

    procedure AssignNoCalc(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('A', 'hello');
        L.Get('A');
        C := L;
        C.K := 'A2';
        C.Insert();
        exit(Show(C) + ' ' + Stored('A2'));
    end;

    procedure SameVariableNoCalc(): Text
    var L: Record "Bi Row";
    begin
        Seed('B', 'hello');
        L.Get('B');
        L.K := 'B2';
        L.Insert();
        exit(Show(L) + ' ' + Stored('B2'));
    end;

    procedure DeleteInsert(): Text
    var L: Record "Bi Row";
    begin
        Seed('B3', 'hello');
        L.Get('B3');
        L.Delete();
        L.Insert();
        exit(Show(L) + ' ' + Stored('B3'));
    end;

    procedure CopyNoCalc(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('D', 'hello');
        L.Get('D');
        C.Copy(L);
        C.K := 'D2';
        C.Insert();
        exit(Show(C) + ' ' + Stored('D2'));
    end;

    procedure InsertTrue(): Text
    var L: Record "Bi Row"; C: Record "Bi Row"; R: Record "Bi Row";
    begin
        Seed('T', 'hello');
        L.Get('T');
        C := L;
        C.K := 'TRG';
        C.Insert(true);
        R.Get('TRG');
        exit(R.Note + ' ' + Show(C) + ' ' + Stored('TRG'));
    end;

    procedure TemporaryNoCalc(): Text
    var T: Record "Bi Row" temporary; OutS: OutStream; Own: Text; Before: Boolean;
    begin
        T.K := 'G';
        T.Body.CreateOutStream(OutS);
        OutS.WriteText('hello');
        T.Insert();
        T.Get('G');
        T.K := 'G2';
        T.Insert();
        Own := Show(T);
        T.Get('G2');
        Before := T.Body.HasValue();
        T.CalcFields(Body);
        exit(StrSubstNo('%1 %2 %3', Own, Before, Show(T)));
    end;

    procedure ModifyOtherRow(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('H', 'hello');
        Seed('H2', 'other');
        L.Get('H');
        C := L;
        C.K := 'H2';
        C.Modify();
        exit(Show(C) + ' ' + Stored('H2'));
    end;

    procedure ModifyOwnRow(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('J', 'hello');
        L.Get('J');
        C := L;
        C.Modify();
        exit(Stored('J'));
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
        exit(Show(C) + ' ' + Stored('E2'));
    end;

    procedure AssignNoCalcStoredEmpty(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('F', '');
        L.Get('F');
        C := L;
        C.K := 'F2';
        C.Insert();
        exit(Show(C) + ' ' + Stored('F2'));
    end;
}
"#;

/// The answer the probe measured for an Insert of a Blob never loaded: the record `Yes[]`, the row `No No[]`.
const NOT_STORED: &str = "Yes[] No No[]";

#[test]
fn an_insert_of_an_assigned_copy_of_a_blob_never_loaded_stores_no_blob() {
    // BlobInsertUnloaded.al C01 (corpus pull request 1631).
    assert_eq!(text(run_in(SRC, "C", "AssignNoCalc")), NOT_STORED);
}

#[test]
fn an_insert_under_a_new_key_of_a_record_whose_blob_was_never_loaded_stores_no_blob() {
    // C02.
    assert_eq!(text(run_in(SRC, "C", "SameVariableNoCalc")), NOT_STORED);
}

#[test]
fn a_delete_then_insert_of_a_blob_never_loaded_leaves_the_row_without_it() {
    // C03.
    assert_eq!(text(run_in(SRC, "C", "DeleteInsert")), NOT_STORED);
}

#[test]
fn an_insert_of_a_record_copy_of_a_blob_never_loaded_stores_no_blob() {
    // C04.
    assert_eq!(text(run_in(SRC, "C", "CopyNoCalc")), NOT_STORED);
}

#[test]
fn an_insert_true_sees_the_blob_before_calcfields_and_not_after() {
    // C09: in OnInsert HasValue is Yes before CalcFields and No after; the record then holds the loaded empty Blob.
    assert_eq!(text(run_in(SRC, "C", "InsertTrue")), "before=Yes after=No No[] No No[]");
}

#[test]
fn a_temporary_row_inserted_from_a_blob_never_loaded_has_a_value_until_calcfields() {
    // T12: the record reads empty; the new row answers HasValue Yes before CalcFields, No and empty after.
    assert_eq!(text(run_in(SRC, "C", "TemporaryNoCalc")), "Yes[] Yes No[]");
}

#[test]
fn a_modify_of_a_copy_of_a_blob_never_loaded_under_another_rows_key_keeps_that_rows_blob() {
    // C11.
    assert_eq!(text(run_in(SRC, "C", "ModifyOtherRow")), "Yes[] Yes Yes[other]");
}

#[test]
fn a_modify_of_a_copy_of_a_blob_never_loaded_on_its_own_row_keeps_the_stored_blob() {
    // streams/BlobCopyStates.al:1219 `bv get row`.
    assert_eq!(text(run_in(SRC, "C", "ModifyOwnRow")), "Yes Yes[hello]");
}

#[test]
fn an_insert_of_an_assigned_copy_of_a_loaded_blob_stores_its_content() {
    // K1.
    assert_eq!(text(run_in(SRC, "C", "AssignWithCalc")), "Yes[hello] Yes Yes[hello]");
}

#[test]
fn an_insert_of_an_assigned_copy_of_a_stored_empty_blob_stores_none() {
    // K3.
    assert_eq!(text(run_in(SRC, "C", "AssignNoCalcStoredEmpty")), "No[] No No[]");
}
