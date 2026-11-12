//! Audit fields assigned by AL code, then written (record/system-fields/AssignedAuditFields.al). The corpus pins the measured
//! shapes; these tests hold the refusals around them and the neighbor shapes that keep their earlier result.

use al_builtins::native::Signal;
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

    // ---- Rename neighbors: each writes the buffer's created pair, as on main.

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
        Row.Get('A');
        Other.Get('A');
        Other.Delete();
        Sleep(20);
        Other.Insert();
        exit(Format(Row.Rename('B')));
    end;

    procedure RenameBothCreatedBlank(): Text
    var
        Row: Record R;
        Stored: Record R;
        NoUser: Guid;
    begin
        Seed(Row);
        Row.Get('A');
        Row.SystemCreatedAt := 0DT;
        Row.SystemCreatedBy := NoUser;
        Row.Rename('B');
        Stored.Get('B');
        exit(Format(Stored.SystemCreatedAt = 0DT));
    end;

    procedure RenameBothCreatedWithOnRename(): Text
    var
        Row: Record RT;
        Stored: Record RT;
    begin
        Row.DeleteAll();
        Row."No." := 'A';
        Row.Insert();
        Row.Get('A');
        Row.SystemCreatedAt := At();
        Row.SystemCreatedBy := By();
        Row.Rename('B');
        Stored.Get('B');
        exit(Format(Stored.SystemCreatedAt = At()));
    end;

    procedure RenameBothCreatedThroughRecordRef(): Text
    var
        Row: Record R;
        Stored: Record R;
        RecRef: RecordRef;
    begin
        Seed(Row);
        Row.Get('A');
        RecRef.GetTable(Row);
        RecRef.Field(RecRef.SystemCreatedAtNo()).Value := At();
        RecRef.Field(RecRef.SystemCreatedByNo()).Value := By();
        RecRef.Rename('B');
        Stored.Get('B');
        exit(Format(Stored.SystemCreatedAt = At()));
    end;

    // ---- Temporary Modify neighbors: the row keeps the stored created pair, as on main.

    local procedure TempSeed(var TempRow: Record R temporary)
    begin
        TempRow."No." := 'A';
        TempRow.SystemCreatedAt := At();
        TempRow.SystemCreatedBy := By();
        TempRow.Insert();
        Clear(TempRow);
        TempRow.Get('A');
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
        exit(Format(TempRow.SystemCreatedBy = By()));
    end;

    procedure TempCreatedAtBlankNothingElse(): Text
    var
        TempRow: Record R temporary;
    begin
        TempSeed(TempRow);
        TempRow.SystemCreatedAt := 0DT;
        TempRow.Modify();
        Clear(TempRow);
        TempRow.Get('A');
        exit(Format(TempRow.SystemCreatedAt = At()));
    end;

    procedure TempCreatedAtBlankRunTrigger(): Text
    var
        TempRow: Record R temporary;
    begin
        TempSeed(TempRow);
        TempRow.SystemCreatedAt := 0DT;
        TempRow.Name := 'second';
        TempRow.Modify(true);
        Clear(TempRow);
        TempRow.Get('A');
        exit(Format(TempRow.SystemCreatedAt = At()));
    end;

    procedure TempCreatedAtBlankPinned(): Text
    var
        TempRow: Record R temporary;
    begin
        TempSeed(TempRow);
        TempRow.SystemCreatedAt := 0DT;
        TempRow.Name := 'second';
        TempRow.Modify();
        Clear(TempRow);
        TempRow.Get('A');
        exit(Format(TempRow.SystemCreatedAt = 0DT));
    end;

    // ---- No-change Modify neighbors: the buffer takes the stored modified pair, as on main.

    procedure NoChangeCreatedAndModifiedAtAssigned(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.Get('A');
        Row.SystemCreatedAt := At();
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(Format(Row.SystemModifiedAt = At()));
    end;

    procedure NoChangeModifiedAtBlank(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.Get('A');
        Row.SystemModifiedAt := 0DT;
        Row.Modify();
        exit(Format(Row.SystemModifiedAt = 0DT));
    end;

    procedure NoChangeThroughRecordRef(): Text
    var
        Row: Record R;
        RecRef: RecordRef;
    begin
        Seed(Row);
        Row.Get('A');
        RecRef.GetTable(Row);
        RecRef.Field(RecRef.SystemModifiedAtNo()).Value := At();
        RecRef.Modify();
        exit(Format(RecRef.Field(RecRef.SystemModifiedAtNo()).Value = At()));
    end;

    procedure NoChangeAfterOwnInsert(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(Format(Row.SystemModifiedAt = At()));
    end;

    procedure NoChangeTableWithOnModify(): Text
    var
        Row: Record RT;
    begin
        Row.DeleteAll();
        Row."No." := 'A';
        Row.Insert();
        Row.Get('A');
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(Format(Row.SystemModifiedAt = At()));
    end;

    procedure NoChangeWithSubscriber(): Text
    var
        Row: Record RS;
    begin
        Row.DeleteAll();
        Row."No." := 'A';
        Row.Insert();
        Row.Get('A');
        Row.SystemModifiedAt := At();
        Row.Modify();
        exit(Format(Row.SystemModifiedAt = At()));
    end;

    procedure NoChangeRunTrigger(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.Get('A');
        Row.SystemModifiedAt := At();
        Row.Modify(true);
        exit(Format(Row.SystemModifiedAt = At()));
    end;

    // ---- ModifyAll shapes the pin does not cover.

    procedure ModifyAllFiltered()
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.SetRange("No.", 'A');
        Row.ModifyAll(SystemCreatedAt, At());
    end;

    procedure ModifyAllTableWithOnModify()
    var
        Row: Record RT;
    begin
        Row.DeleteAll();
        Row."No." := 'A';
        Row.Insert();
        Row.ModifyAll(SystemCreatedAt, At());
    end;

    procedure ModifyAllWithSubscriber()
    var
        Row: Record RS;
    begin
        Row.DeleteAll();
        Row."No." := 'A';
        Row.Insert();
        Row.ModifyAll(SystemCreatedAt, At());
    end;

    local procedure Seed(var Row: Record R)
    begin
        Row.DeleteAll();
        Row."No." := 'A';
        Row.Insert();
    end;

    procedure ModifyAllCreatedAt(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        asserterror Row.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure ModifyAllModifiedBy()
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.ModifyAll(SystemModifiedBy, CreateGuid());
    end;

    procedure ModifyAllCreatedAtNoRows()
    var
        Row: Record R;
    begin
        Row.DeleteAll();
        Row.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure ModifyAllCreatedAtRunTrigger()
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T), true);
    end;

    procedure ModifyAllCreatedAtTemporary()
    var
        Row: Record R temporary;
    begin
        Row."No." := 'A';
        Row.Insert();
        Row.ModifyAll(SystemCreatedAt, CreateDateTime(20200102D, 120000T));
    end;

    procedure RenameOneCreatedAssigned()
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.Get('A');
        Row.SystemCreatedAt := CreateDateTime(20200102D, 120000T);
        Row.Rename('B');
    end;

    // Neighbor: a plain Rename keeps main's result.
    procedure RenamePlain(): Text
    var
        Row: Record R;
    begin
        Seed(Row);
        Row.Get('A');
        Row.Rename('B');
        exit(Format(Row.Get('B')));
    end;

    // Neighbor: a stale buffer (another variable changed the row) with nothing changed keeps the earlier result: the buffer
    // takes the stored modified pair.
    procedure StaleBufferNoChange(): Text
    var
        Row: Record R;
        Other: Record R;
    begin
        Seed(Row);
        Row.Get('A');
        Other.Get('A');
        Sleep(20);
        Other.Name := 'x';
        Other.Modify();
        Row.Name := 'x';
        Row.Modify();
        Other.Get('A');
        exit(Format(Row.SystemModifiedAt = Other.SystemModifiedAt));
    end;

    // The pinned shape: SystemModifiedAt assigned, nothing else changed; the buffer keeps it, the stored row does not.
    procedure AssignedModifiedAtNoChange(): Text
    var
        Row: Record R;
        Stored: Record R;
        At: DateTime;
    begin
        Seed(Row);
        Row.Get('A');
        At := CreateDateTime(20200102D, 120000T);
        Row.SystemModifiedAt := At;
        Row.Modify();
        Stored.Get('A');
        exit(Format(Row.SystemModifiedAt = At) + '|' + Format(Stored.SystemModifiedAt = At));
    end;
}
"#;

fn refused(r: Result<al_builtins::value::Value, Signal>) -> bool {
    matches!(r, Err(Signal::Unsupported(_)))
}

#[test]
fn modifyall_of_a_pinned_audit_field_raises_the_pinned_text() {
    assert_eq!(
        text(run_in(SRC, "T", "ModifyAllCreatedAt")),
        "You cannot change the value of the SystemCreatedAt field.\\\\Table: R|DB:Ok"
    );
}

#[test]
fn audit_field_writes_outside_the_pinned_shapes_are_refused() {
    for routine in [
        "ModifyAllModifiedBy",
        "ModifyAllCreatedAtNoRows",
        "ModifyAllCreatedAtRunTrigger",
        "ModifyAllCreatedAtTemporary",
        "RenameOneCreatedAssigned",
        "ModifyAllFiltered",
        "ModifyAllTableWithOnModify",
        "ModifyAllWithSubscriber",
    ] {
        assert!(refused(run_in(SRC, "T", routine)), "{routine} must be refused");
    }
}

#[test]
fn a_no_change_modify_keeps_the_assigned_modified_at_only_in_the_buffer() {
    assert_eq!(text(run_in(SRC, "T", "AssignedModifiedAtNoChange")), "Yes|No");
}

#[test]
fn a_stale_buffer_without_change_keeps_the_earlier_result() {
    assert_eq!(text(run_in(SRC, "T", "StaleBufferNoChange")), "Yes");
}

#[test]
fn a_plain_rename_keeps_working() {
    assert_eq!(text(run_in(SRC, "T", "RenamePlain")), "Yes");
}

/// A Rename whose buffer is not the stored row's own image, or that the pin does not cover, writes the buffer's created pair.
#[test]
fn rename_neighbors_write_the_buffer_created_pair() {
    for routine in [
        "RenameBufferCopiedFromOtherRow",
        "RenameRowReinsertedByOtherVariable",
        "RenameBothCreatedBlank",
        "RenameBothCreatedWithOnRename",
        "RenameBothCreatedThroughRecordRef",
    ] {
        assert_eq!(text(run_in(SRC, "T", routine)), "Yes", "{routine}");
    }
}

/// Only the pinned temporary cell writes the buffer's SystemCreatedAt; the neighbors keep the stored created pair.
#[test]
fn temporary_modify_neighbors_keep_the_stored_created_pair() {
    assert_eq!(text(run_in(SRC, "T", "TempCreatedAtBlankPinned")), "Yes");
    for routine in [
        "TempCreatedByBlank",
        "TempCreatedAtBlankNothingElse",
        "TempCreatedAtBlankRunTrigger",
    ] {
        assert_eq!(text(run_in(SRC, "T", routine)), "Yes", "{routine}");
    }
}

/// A no-change Modify outside the pinned cell gives the buffer the stored modified pair.
#[test]
fn no_change_modify_neighbors_take_the_stored_modified_pair() {
    for routine in [
        "NoChangeCreatedAndModifiedAtAssigned",
        "NoChangeModifiedAtBlank",
        "NoChangeThroughRecordRef",
        "NoChangeAfterOwnInsert",
        "NoChangeTableWithOnModify",
        "NoChangeWithSubscriber",
        "NoChangeRunTrigger",
    ] {
        assert_eq!(text(run_in(SRC, "T", routine)), "No", "{routine}");
    }
}
