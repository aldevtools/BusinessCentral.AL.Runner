//! Audit fields assigned by AL code, then written (record/system-fields/AssignedAuditFields.al). The corpus pins the measured
//! cells; these tests repeat each cell once and hold the neighbor shapes, each one fact away from its cell, at the result
//! they had before the rules.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50330 R
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; "No.") { } }
}

table 50332 RT
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; "No.") { } }

    trigger OnModify() begin end;
    trigger OnRename() begin end;
}

table 50333 RS
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; "No.") { } }
}

codeunit 50334 Sub
{
    [EventSubscriber(ObjectType::Table, Database::RS, 'OnAfterModifyEvent', '', false, false)]
    local procedure AfterModify(var Rec: Record RS) begin end;
}

codeunit 50331 T
{
    local procedure At(): DateTime
    begin
        exit(CreateDateTime(20200102D, 120000T));
    end;

    local procedure By(): Guid
    var
        Id: Guid;
    begin
        Evaluate(Id, '{11111111-2222-3333-4444-555555555555}');
        exit(Id);
    end;

    // AssignedAuditFields.al:140-148.
    local procedure Seed(var Row: Record R)
    begin
        Row.DeleteAll();
        Clear(Row);
        Row."No." := 'A';
        Row.Name := 'first';
        Row.Insert();
        Row.Get('A');
    end;

    local procedure SeedNoGet(var Row: Record R)
    begin
        Row.DeleteAll();
        Clear(Row);
        Row."No." := 'A';
        Row.Name := 'first';
        Row.Insert();
    end;

    // What a Modify left: the buffer holds the assigned SystemModifiedAt | the stored row holds it.
    local procedure AfterModify(var Row: Record R): Text
    var
        Stored: Record R;
    begin
        Stored.Get('A');
        exit(Format(Row.SystemModifiedAt = At()) + '|' + Format(Stored.SystemModifiedAt = At()));
    end;

    // ---- The no-change Modify cell (:271-283) and its neighbors.

    procedure ModifyPinned(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyTwice(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemModifiedAt := At();
        Row.Modify();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyThenAssignAgain(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemModifiedAt := CreateDateTime(20210102D, 120000T);
        Row.Modify();
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyCommitAfterGet(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Commit();
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyCommitAfterAssign(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemModifiedAt := At();
        Commit();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyCommitBeforeGet(): Text
    var
        Row: Record R;
    begin
        SeedNoGet(Row);
        Commit();
        Row.Get('A');
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyCopy(): Text
    var
        Row: Record R;
        Copy: Record R;
    begin
        Seed(Row);
        Copy := Row;
        Copy.SystemModifiedAt := At();
        Copy.Modify();
        exit(AfterModify(Copy));
    end;

    procedure ModifyAfterFindFirst(): Text
    var
        Row: Record R;
    begin
        SeedNoGet(Row);
        Row.FindFirst();
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyAfterGetBySystemId(): Text
    var
        Row: Record R;
    begin
        SeedNoGet(Row);
        Row.GetBySystemId(Row.SystemId);
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyOtherVariableGet(): Text
    var
        Row: Record R;
        Other: Record R;
    begin
        Seed(Row);
        Other.Get('A');
        Other.SystemModifiedAt := At();
        Other.Modify();
        exit(AfterModify(Other));
    end;

    procedure ModifySecondGet(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.Get('A');
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyNoGet(): Text
    var
        Row: Record R;
    begin
        SeedNoGet(Row);
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyInsertRunTrigger(): Text
    var
        Row: Record R;
    begin
        Row.DeleteAll();
        Clear(Row);
        Row."No." := 'A';
        Row.Insert(true);
        Row.Get('A');
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyClearBeforeGet(): Text
    var
        Row: Record R;
    begin
        SeedNoGet(Row);
        Clear(Row);
        Row.Get('A');
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyResetAfterGet(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.Reset();
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyFilterAfterGet(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SetRange("No.", 'A');
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyFilterBeforeInsert(): Text
    var
        Row: Record R;
    begin
        Row.DeleteAll();
        Clear(Row);
        Row.SetRange("No.", 'A');
        Row."No." := 'A';
        Row.Insert();
        Row.Get('A');
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyOtherRowWrittenAfterGet(): Text
    var
        Row: Record R;
        Other: Record R;
    begin
        Seed(Row);
        Other."No." := 'B';
        Other.Insert();
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyBothAtAssigned(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemCreatedAt := At();
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyBothModifiedAssigned(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemModifiedAt := At();
        Row.SystemModifiedBy := By();
        Row.Modify();
        exit(AfterModify(Row));
    end;

    procedure ModifyBlank(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemModifiedAt := 0DT;
        Row.Modify();
        exit(Format(Row.SystemModifiedAt = 0DT));
    end;

    procedure ModifyRunTrigger(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemModifiedAt := At();
        Row.Modify(true);
        exit(AfterModify(Row));
    end;

    procedure ModifyThroughRecordRef(): Text
    var
        Row: Record R;
        RecRef: RecordRef;
    begin
        Seed(Row);
        RecRef.GetTable(Row);
        RecRef.Field(RecRef.SystemModifiedAtNo()).Value := At();
        RecRef.Modify();
        exit(Format(RecRef.Field(RecRef.SystemModifiedAtNo()).Value = At()));
    end;

    procedure ModifyTableWithOnModify(): Text
    var
        Row: Record RT;
    begin
        Row.DeleteAll();
        Clear(Row);
        Row."No." := 'A';
        Row.Insert();
        Row.Get('A');
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(Format(Row.SystemModifiedAt = At()));
    end;

    procedure ModifyWithSubscriber(): Text
    var
        Row: Record RS;
    begin
        Row.DeleteAll();
        Clear(Row);
        Row."No." := 'A';
        Row.Insert();
        Row.Get('A');
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(Format(Row.SystemModifiedAt = At()));
    end;

    // A stale buffer (another variable changed the row) with nothing changed: the buffer takes the stored modified pair.
    procedure ModifyStaleBuffer(): Text
    var
        Row: Record R;
        Other: Record R;
    begin
        Seed(Row);
        Other.Get('A');
        Sleep(20);
        Other.Name := 'x';
        Other.Modify();
        Row.Name := 'x';
        Row.Modify();
        Other.Get('A');
        exit(Format(Row.SystemModifiedAt = Other.SystemModifiedAt));
    end;

    // ---- The Rename cell (:568-590) and its neighbors. The answer: the stored row holds the assigned SystemCreatedAt.

    local procedure AfterRename(): Text
    var
        Stored: Record R;
    begin
        Stored.Get('B');
        exit(Format(Stored.SystemCreatedAt = At()));
    end;

    procedure RenamePinned(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemCreatedAt := At();
        Row.SystemCreatedBy := By();
        Row.Rename('B');
        exit(AfterRename());
    end;

    procedure RenameOneCreatedAssigned(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemCreatedAt := At();
        Row.Rename('B');
        exit(AfterRename());
    end;

    procedure RenameNameChanged(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemCreatedAt := At();
        Row.SystemCreatedBy := By();
        Row.Name := 'x';
        Row.Rename('B');
        exit(AfterRename());
    end;

    procedure RenameCommitAfterGet(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Commit();
        Row.SystemCreatedAt := At();
        Row.SystemCreatedBy := By();
        Row.Rename('B');
        exit(AfterRename());
    end;

    procedure RenameCopy(): Text
    var
        Row: Record R;
        Copy: Record R;
    begin
        Seed(Row);
        Copy := Row;
        Copy.SystemCreatedAt := At();
        Copy.SystemCreatedBy := By();
        Copy.Rename('B');
        exit(AfterRename());
    end;

    procedure RenameNoGet(): Text
    var
        Row: Record R;
    begin
        SeedNoGet(Row);
        Row.SystemCreatedAt := At();
        Row.SystemCreatedBy := By();
        Row.Rename('B');
        exit(AfterRename());
    end;

    procedure RenameAfterModify(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.Modify();
        Row.SystemCreatedAt := At();
        Row.SystemCreatedBy := By();
        Row.Rename('B');
        exit(AfterRename());
    end;

    procedure RenameAllFourAssigned(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemCreatedAt := At();
        Row.SystemCreatedBy := By();
        Row.SystemModifiedAt := At();
        Row.SystemModifiedBy := By();
        Row.Rename('B');
        exit(AfterRename());
    end;

    procedure RenameBothCreatedBlank(): Text
    var
        Row: Record R;
        Stored: Record R;
        NoUser: Guid;
    begin
        Seed(Row);
        Row.SystemCreatedAt := 0DT;
        Row.SystemCreatedBy := NoUser;
        Row.Rename('B');
        Stored.Get('B');
        exit(Format(Stored.SystemCreatedAt = 0DT));
    end;

    procedure RenameTableWithOnRename(): Text
    var
        Row: Record RT;
        Stored: Record RT;
    begin
        Row.DeleteAll();
        Clear(Row);
        Row."No." := 'A';
        Row.Insert();
        Row.Get('A');
        Row.SystemCreatedAt := At();
        Row.SystemCreatedBy := By();
        Row.Rename('B');
        Stored.Get('B');
        exit(Format(Stored.SystemCreatedAt = At()));
    end;

    procedure RenameThroughRecordRef(): Text
    var
        Row: Record R;
        RecRef: RecordRef;
    begin
        Seed(Row);
        RecRef.GetTable(Row);
        RecRef.Field(RecRef.SystemCreatedAtNo()).Value := At();
        RecRef.Field(RecRef.SystemCreatedByNo()).Value := By();
        RecRef.Rename('B');
        exit(AfterRename());
    end;

    procedure RenameBufferCopiedFromOtherRow(): Text
    var
        Source: Record R;
        Target: Record R;
        Stored: Record R;
    begin
        Source.DeleteAll();
        Source."No." := 'A';
        Source.Insert();
        Source.Get('A');
        Sleep(20);
        Target."No." := 'B';
        Target.Insert();
        Target := Source;
        Target."No." := 'B';
        Target.Rename('C');
        Stored.Get('C');
        exit(Format(Stored.SystemCreatedAt = Source.SystemCreatedAt));
    end;

    procedure RenameRowReinsertedByOtherVariable(): Text
    var
        Row: Record R;
        Other: Record R;
    begin
        Seed(Row);
        Other.Get('A');
        Other.Delete();
        Sleep(20);
        Other.Insert();
        exit(Format(Row.Rename('B')));
    end;

    procedure RenamePlain(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.Rename('B');
        exit(Format(Row.Get('B')));
    end;

    // ---- ModifyAll on an audit field: no rule of this file covers it.

    procedure ModifyAllCreatedAt(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.Reset();
        Row.ModifyAll(SystemCreatedAt, At());
        exit('no error');
    end;

    procedure ModifyAllModifiedBy(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.Reset();
        Row.ModifyAll(SystemModifiedBy, By());
        exit('no error');
    end;

    procedure ModifyAllNoRows(): Text
    var
        Row: Record R;
    begin
        Row.DeleteAll();
        Row.ModifyAll(SystemCreatedAt, At());
        exit('no error');
    end;

    procedure ModifyAllTableWithOnModify(): Text
    var
        Row: Record RT;
    begin
        Row.DeleteAll();
        Clear(Row);
        Row."No." := 'A';
        Row.Insert();
        Row.ModifyAll(SystemCreatedAt, At(), true);
        exit('no error');
    end;

    // ---- The temporary Modify cell (:638-666) and its neighbors. The answer: the stored row holds a blank SystemCreatedAt.

    // :645-655.
    local procedure TempSeed(var TempRow: Record R temporary)
    begin
        TempRow."No." := 'A';
        TempRow.SystemCreatedAt := At();
        TempRow.SystemCreatedBy := By();
        TempRow.SystemModifiedAt := At();
        TempRow.SystemModifiedBy := By();
        TempRow.Insert();
        Clear(TempRow);
        TempRow.Get('A');
    end;

    local procedure TempAfterModify(var TempRow: Record R temporary): Text
    begin
        Clear(TempRow);
        TempRow.Get('A');
        exit(Format(TempRow.SystemCreatedAt = 0DT));
    end;

    procedure TempPinned(): Text
    var
        TempRow: Record R temporary;
    begin
        TempSeed(TempRow);
        TempRow.SystemCreatedAt := 0DT;
        TempRow.Name := 'second';
        TempRow.Modify();
        exit(TempAfterModify(TempRow));
    end;

    procedure TempNoClearBeforeGet(): Text
    var
        TempRow: Record R temporary;
    begin
        TempRow."No." := 'A';
        TempRow.SystemCreatedAt := At();
        TempRow.Insert();
        TempRow.Get('A');
        TempRow.SystemCreatedAt := 0DT;
        TempRow.Name := 'second';
        TempRow.Modify();
        exit(TempAfterModify(TempRow));
    end;

    procedure TempNoGet(): Text
    var
        TempRow: Record R temporary;
    begin
        TempRow."No." := 'A';
        TempRow.SystemCreatedAt := At();
        TempRow.Insert();
        TempRow.SystemCreatedAt := 0DT;
        TempRow.Name := 'second';
        TempRow.Modify();
        exit(TempAfterModify(TempRow));
    end;

    procedure TempSecondModify(): Text
    var
        TempRow: Record R temporary;
    begin
        TempSeed(TempRow);
        TempRow.Modify();
        TempRow.SystemCreatedAt := 0DT;
        TempRow.Name := 'second';
        TempRow.Modify();
        exit(TempAfterModify(TempRow));
    end;

    procedure TempCopy(): Text
    var
        TempRow: Record R temporary;
        TempCopy: Record R temporary;
    begin
        TempSeed(TempRow);
        TempCopy.Copy(TempRow, true);
        TempCopy.SystemCreatedAt := 0DT;
        TempCopy.Name := 'second';
        TempCopy.Modify();
        exit(TempAfterModify(TempRow));
    end;

    procedure TempNothingElseChanged(): Text
    var
        TempRow: Record R temporary;
    begin
        TempSeed(TempRow);
        TempRow.SystemCreatedAt := 0DT;
        TempRow.Modify();
        exit(TempAfterModify(TempRow));
    end;

    procedure TempRunTrigger(): Text
    var
        TempRow: Record R temporary;
    begin
        TempSeed(TempRow);
        TempRow.SystemCreatedAt := 0DT;
        TempRow.Name := 'second';
        TempRow.Modify(true);
        exit(TempAfterModify(TempRow));
    end;

    procedure TempCreatedByBlank(): Text
    var
        TempRow: Record R temporary;
        NoUser: Guid;
    begin
        TempSeed(TempRow);
        TempRow.SystemCreatedBy := NoUser;
        TempRow.Name := 'second';
        TempRow.Modify();
        Clear(TempRow);
        TempRow.Get('A');
        exit(Format(IsNullGuid(TempRow.SystemCreatedBy)));
    end;

    procedure TempCreatedAtOtherValue(): Text
    var
        TempRow: Record R temporary;
    begin
        TempSeed(TempRow);
        TempRow.SystemCreatedAt := CreateDateTime(20210102D, 120000T);
        TempRow.Name := 'second';
        TempRow.Modify();
        Clear(TempRow);
        TempRow.Get('A');
        exit(Format(TempRow.SystemCreatedAt = At()));
    end;
}
"#;

fn answers(routines: &[&str], expected: &str) {
    for routine in routines {
        assert_eq!(text(run_in(SRC, "T", routine)), expected, "{routine}");
    }
}

/// AssignedAuditFields.al:271-283: the buffer keeps the assigned SystemModifiedAt, the stored row does not take it.
#[test]
fn the_no_change_modify_cell_keeps_the_assigned_value_in_the_buffer_only() {
    answers(&["ModifyPinned"], "Yes|No");
}

/// One fact away from the cell: the buffer takes the stored modified pair, as before the rule.
#[test]
fn no_change_modify_neighbors_take_the_stored_modified_pair() {
    answers(
        &[
            "ModifyTwice",
            "ModifyThenAssignAgain",
            "ModifyCommitAfterGet",
            "ModifyCommitAfterAssign",
            "ModifyCommitBeforeGet",
            "ModifyCopy",
            "ModifyAfterFindFirst",
            "ModifyAfterGetBySystemId",
            "ModifyOtherVariableGet",
            "ModifySecondGet",
            "ModifyNoGet",
            "ModifyInsertRunTrigger",
            "ModifyClearBeforeGet",
            "ModifyResetAfterGet",
            "ModifyFilterAfterGet",
            "ModifyFilterBeforeInsert",
            "ModifyOtherRowWrittenAfterGet",
            "ModifyBothAtAssigned",
            "ModifyBothModifiedAssigned",
            "ModifyRunTrigger",
        ],
        "No|No",
    );
    answers(
        &[
            "ModifyBlank",
            "ModifyThroughRecordRef",
            "ModifyTableWithOnModify",
            "ModifyWithSubscriber",
        ],
        "No",
    );
    answers(&["ModifyStaleBuffer"], "Yes");
}

/// AssignedAuditFields.al:568-590: the stored row keeps its created pair.
#[test]
fn the_rename_cell_keeps_the_stored_created_pair() {
    answers(&["RenamePinned"], "No");
}

/// One fact away from the cell: the Rename writes the buffer's created pair, as before the rule.
#[test]
fn rename_neighbors_write_the_buffer_created_pair() {
    answers(
        &[
            "RenameOneCreatedAssigned",
            "RenameNameChanged",
            "RenameCommitAfterGet",
            "RenameCopy",
            "RenameNoGet",
            "RenameAfterModify",
            "RenameAllFourAssigned",
            "RenameBothCreatedBlank",
            "RenameTableWithOnRename",
            "RenameThroughRecordRef",
            "RenameBufferCopiedFromOtherRow",
            "RenameRowReinsertedByOtherVariable",
            "RenamePlain",
        ],
        "Yes",
    );
}

/// ModifyAll on an audit field runs as before: the measured cell (AssignedAuditFields.al:594-632) is not built, see the
/// module doc of `assigned_audit.rs`.
#[test]
fn modifyall_of_an_audit_field_raises_no_error() {
    answers(
        &[
            "ModifyAllCreatedAt",
            "ModifyAllModifiedBy",
            "ModifyAllNoRows",
            "ModifyAllTableWithOnModify",
        ],
        "no error",
    );
}

/// AssignedAuditFields.al:638-666: the row takes the blank SystemCreatedAt.
#[test]
fn the_temporary_modify_cell_writes_the_blank_created_at() {
    answers(&["TempPinned"], "Yes");
}

/// One fact away from the cell: the row keeps the stored created pair, as before the rule.
#[test]
fn temporary_modify_neighbors_keep_the_stored_created_pair() {
    answers(
        &[
            "TempNoClearBeforeGet",
            "TempNoGet",
            "TempSecondModify",
            "TempCopy",
            "TempNothingElseChanged",
            "TempRunTrigger",
            "TempCreatedByBlank",
        ],
        "No",
    );
    answers(&["TempCreatedAtOtherValue"], "Yes");
}
