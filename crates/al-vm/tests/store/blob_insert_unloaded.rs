//! A Blob whose stored content the record never loaded (a read without `CalcFields`, or a copy of such a record) and
//! that no stream wrote since is never written by Insert or Modify (issue 9286). The open corpus probe (pull request
//! 1631, record/lt-blob-field-assign/BlobInsertUnloaded.al, 31 tests measured on BC 29.0; its CI runs codeunit 73781 on
//! BC 27.5, 28.5 and 29.0 and asserts codeunit 73782 on BC 28.5 and 29.0) gives the readings: codeunit 73781 the Insert
//! shapes, Insert(true), temporary tables and a Modify onto another row; codeunit 73782 a Modify of the key it was read
//! under, a stale buffer, Insert then Modify, database to temporary and back, a field assignment (also into another
//! table), CalcFields right after the Insert, another company's source (also without switching back) and a row another
//! variable cleared. Through the assumption `blob-never-loaded-not-stored`. A loaded Blob (`CalcFields` before the copy
//! or on it) and an empty source store as before.

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
table 50132 "Bi Other"
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

    // Codeunit 73782 of BlobInsertUnloaded.al (corpus pull request 1631).
    procedure StaleModify(): Text
    var L: Record "Bi Row"; X: Record "Bi Row"; OutS: OutStream;
    begin
        Seed('S', 'hello');
        L.Get('S');
        X.Get('S');
        X.Body.CreateOutStream(OutS);
        OutS.WriteText('zz');
        X.Modify();
        L.Note := 'n';
        L.Modify();
        exit(Stored('S'));
    end;

    procedure InsertThenModify(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('M', 'hello');
        L.Get('M');
        C := L;
        C.K := 'M2';
        C.Insert();
        C.Note := 'n';
        C.Modify();
        exit(Stored('M2'));
    end;

    procedure DatabaseIntoTemporary(): Text
    var L: Record "Bi Row"; T: Record "Bi Row" temporary;
    begin
        Seed('P', 'hello');
        L.Get('P');
        T := L;
        T.K := 'P2';
        T.Insert();
        T.Get('P2');
        T.CalcFields(Body);
        exit(Show(T));
    end;

    procedure TemporaryIntoDatabase(): Text
    var T: Record "Bi Row" temporary; C: Record "Bi Row"; OutS: OutStream;
    begin
        T.K := 'Q';
        T.Body.CreateOutStream(OutS);
        OutS.WriteText('hello');
        T.Insert();
        T.Get('Q');
        C := T;
        C.K := 'Q2';
        C.Insert();
        exit(Stored('Q2'));
    end;

    procedure FieldAssignment(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('U', 'hello');
        L.Get('U');
        C.K := 'U2';
        C.Body := L.Body;
        C.Insert();
        exit(Stored('U2'));
    end;

    // R1 and R2 of codeunit 73782.
    procedure ClearedRowModify(): Text
    var L: Record "Bi Row"; X: Record "Bi Row";
    begin
        Seed('X', 'hello');
        L.Get('X');
        X.Get('X');
        Clear(X.Body);
        X.Modify();
        L.Note := 'n';
        L.Modify();
        exit(Stored('X'));
    end;

    procedure FieldAssignOtherTable(): Text
    var L: Record "Bi Row"; B: Record "Bi Other"; R: Record "Bi Other";
    begin
        Seed('Y', 'hello');
        L.Get('Y');
        B.K := 'Y2';
        B.Body := L.Body;
        B.Insert();
        R.Get('Y2');
        R.CalcFields(Body);
        exit(Format(R.Body.HasValue()));
    end;

    procedure OwnCalcFields(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('W', 'hello');
        L.Get('W');
        C := L;
        C.K := 'W2';
        C.Insert();
        C.CalcFields(Body);
        exit(Show(C));
    end;

    procedure ChangeCompanySource(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('V', 'hello');
        L.ChangeCompany(CompanyName());
        L.Get('V');
        C := L;
        C.K := 'V2';
        C.Insert();
        exit(Stored('V2'));
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

#[test]
fn a_modify_of_a_stale_buffer_keeps_the_other_variables_newer_write() {
    // M2: the unloaded Blob does not overwrite the row.
    assert_eq!(text(run_in(SRC, "C", "StaleModify")), "Yes Yes[zz]");
}

#[test]
fn a_modify_after_the_insert_of_the_same_variable_stores_no_blob() {
    // M3.
    assert_eq!(text(run_in(SRC, "C", "InsertThenModify")), "No No[]");
}

#[test]
fn a_database_row_copied_into_a_temporary_record_has_a_value_until_calcfields() {
    // I1: the temporary row reads HasValue No and empty after CalcFields.
    assert_eq!(text(run_in(SRC, "C", "DatabaseIntoTemporary")), "No[]");
}

#[test]
fn a_temporary_row_copied_into_a_database_record_stores_no_blob() {
    // I2.
    assert_eq!(text(run_in(SRC, "C", "TemporaryIntoDatabase")), "No No[]");
}

#[test]
fn a_blob_field_assignment_stores_no_blob() {
    // I3 (record/flowfields/BlobCalcFieldsLength.al:404: a field assignment of an unloaded Blob gives an empty one).
    assert_eq!(text(run_in(SRC, "C", "FieldAssignment")), "No No[]");
}

#[test]
fn a_source_read_after_changecompany_stores_no_blob() {
    // G1.
    assert_eq!(text(run_in(SRC, "C", "ChangeCompanySource")), "No No[]");
}

#[test]
fn calcfields_on_the_inserting_record_right_after_the_insert_has_a_value_and_reads_empty() {
    // I4.
    assert_eq!(text(run_in(SRC, "C", "OwnCalcFields")), "Yes[]");
}

#[test]
fn a_modify_over_a_row_another_variable_cleared_leaves_it_cleared() {
    // R1: the unloaded Blob neither restores nor overwrites the row.
    assert_eq!(text(run_in(SRC, "C", "ClearedRowModify")), "No No[]");
}

#[test]
fn a_blob_field_assignment_into_another_table_stores_no_blob() {
    // R2 (and BlobCalcFieldsLength.al:404: a field assignment of an unloaded Blob gives an empty one, as on main).
    assert_eq!(text(run_in(SRC, "C", "FieldAssignOtherTable")), "No");
}
