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

codeunit 50331 T
{
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
