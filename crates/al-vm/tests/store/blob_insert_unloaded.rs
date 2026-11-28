//! A Blob whose stored content the record never loaded (a read without `CalcFields`, or a copy of such a record) and
//! that no stream wrote since is never written by Insert or Modify (issue 9286). The corpus probe of pull request 1631
//! (merged, not pinned yet: record/lt-blob-field-assign/BlobInsertUnloaded.al, 31 tests, measured on a local Business
//! Central and on every supported major by the corpus CI) gives the readings: codeunit 73781 the Insert
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

    // Codeunit 73783 of BlobInsertUnloadedOrigin.al (corpus pull request 1654): a failed Insert leaves no mark (F4, F2), and
    // the scope of I4 (S2, S4, S5, S6).
    procedure GuardedInsertThenModify(): Text
    var L: Record "Bi Row"; C: Record "Bi Row";
    begin
        Seed('FA', 'hello');
        Seed('FB', 'other');
        L.Get('FA');
        C := L;
        C.K := 'FB';
        if C.Insert() then
            Error('the Insert went through');
        C.CalcFields(Body);
        exit(Show(C));
    end;

    procedure GuardedInsertOwnKey(): Text
    var L: Record "Bi Row";
    begin
        Seed('FC', 'hello');
        L.Get('FC');
        if not L.Insert() then;
        L.CalcFields(Body);
        exit(Show(L));
    end;

    local procedure CopyInsert(var C: Record "Bi Row"; From: Code[20]; Into: Code[20])
    var L: Record "Bi Row";
    begin
        Seed(From, 'hello');
        L.Get(From);
        C := L;
        C.K := Into;
        C.Insert();
    end;

    procedure OtherWriteThenCalcFields(): Text
    var C: Record "Bi Row"; X: Record "Bi Row"; OutS: OutStream;
    begin
        CopyInsert(C, 'SA', 'SA2');
        X.Get('SA2');
        X.Body.CreateOutStream(OutS);
        OutS.WriteText('x');
        X.Modify();
        C.CalcFields(Body);
        exit(Show(C));
    end;

    procedure DeleteThenCalcFields(): Text
    var C: Record "Bi Row";
    begin
        CopyInsert(C, 'SB', 'SB2');
        C.Delete();
        C.CalcFields(Body);
        exit(Show(C));
    end;

    procedure GetThenCalcFields(): Text
    var C: Record "Bi Row";
    begin
        CopyInsert(C, 'SC', 'SC2');
        C.Get('SC2');
        C.CalcFields(Body);
        exit(Show(C));
    end;

    procedure SecondInsertThenCalcFields(): Text
    var C: Record "Bi Row";
    begin
        CopyInsert(C, 'SD', 'SD2');
        C.K := 'SD3';
        C.Insert();
        C.CalcFields(Body);
        exit(Show(C) + ' ' + Stored('SD3'));
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

#[test]
fn a_guarded_insert_onto_an_existing_key_leaves_no_mark() {
    // F4: the Insert fails and CalcFields reads the other row's Blob. (F1, with a Modify in between, meets an existing
    // refusal of a Modify after a failed Insert in the same transaction, on main as well; the neighbor app runs it.)
    assert_eq!(text(run_in(SRC, "C", "GuardedInsertThenModify")), "Yes[other]");
}

#[test]
fn a_guarded_insert_on_its_own_key_leaves_no_mark() {
    // F2.
    assert_eq!(text(run_in(SRC, "C", "GuardedInsertOwnKey")), "Yes[hello]");
}

#[test]
fn another_variables_write_ends_the_unloaded_view_of_that_blob() {
    // S2.
    assert_eq!(text(run_in(SRC, "C", "OtherWriteThenCalcFields")), "Yes[x]");
}

#[test]
fn a_delete_ends_the_unloaded_view() {
    // S4.
    assert_eq!(text(run_in(SRC, "C", "DeleteThenCalcFields")), "No[]");
}

#[test]
fn a_get_ends_the_unloaded_view() {
    // S5.
    assert_eq!(text(run_in(SRC, "C", "GetThenCalcFields")), "No[]");
}

#[test]
fn a_second_insert_stores_no_blob_and_keeps_the_unloaded_view() {
    // S6.
    assert_eq!(text(run_in(SRC, "C", "SecondInsertThenCalcFields")), "Yes[] No No[]");
}

const TWO_TESTS: &str = r#"
table 50133 "Bi Flag Row"
{
    fields { field(1; K; Code[20]) { } field(2; Body; Blob) { } }
    keys { key(PK; K) { } }
}
codeunit 50134 "Bi Flag Tests"
{
    Subtype = Test;

    [Test]
    procedure InsertCopy()
    var L: Record "Bi Flag Row"; C: Record "Bi Flag Row"; OutS: OutStream;
    begin
        L.K := 'A';
        L.Body.CreateOutStream(OutS);
        OutS.WriteText('hello');
        L.Insert();
        L.Get('A');
        C := L;
        C.K := 'B';
        C.Insert();
    end;

    [Test]
    procedure ReadTheRow()
    var R: Record "Bi Flag Row";
    begin
        R.Get('B');
        if R.Body.HasValue() then Error('the row holds a Blob');
    end;

    [Test]
    procedure ReadAnotherRow()
    var R: Record "Bi Flag Row";
    begin
        R.Get('A');
        R.CalcFields(Body);
        if not R.Body.HasValue() then Error('the source row lost its Blob');
    end;
}
"#;

#[test]
fn a_read_of_a_row_the_assumption_stored_without_a_blob_is_flagged_in_a_later_test() {
    use al_builtins::assumptions::Assumption::BlobNeverLoadedNotStored as A;
    let outcomes = al_testkit::outcomes_and_marks_in(TWO_TESTS, "Bi Flag Tests");
    let marks = |name: &str| {
        let (_, outcome, marks) = outcomes.iter().find(|(n, _, _)| n == name).expect("the test ran");
        assert!(matches!(outcome, al_testrun::Outcome::Pass), "{name}: {outcome:?}");
        marks.contains(&A)
    };
    assert!(marks("InsertCopy"));
    assert!(marks("ReadTheRow"));
    assert!(!marks("ReadAnotherRow"));
}
