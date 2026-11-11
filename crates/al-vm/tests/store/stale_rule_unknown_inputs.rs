//! The stale-buffer rule (`records/stale_rule.rs`) decides only what record/crud/StaleRuleMatrix.al and the earlier pins
//! measured. Every other input refuses the write; these shapes wait for corpus PR 1313 (record/crud/StaleRuleEdges.al).

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 50190 "SR Row" { fields { field(1; "No."; Integer) { } field(2; Note; Text[30]) { } field(3; Qty; Integer) { } }
    keys { key(PK; "No.") { } } }
table 50191 "SR Other" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
codeunit 50190 C
{
    // Own Insert and Modify of row 6, row 6 changed by another variable, two Commits: Row is stale.
    local procedure Prepare(var Row: Record "SR Row")
    var Other: Record "SR Row"; Neighbour: Record "SR Row";
    begin
        Commit();
        Neighbour."No." := 8; Neighbour.Insert();
        Row."No." := 6; Row.Note := 'start'; Row.Insert(); Row.Qty := 1; Row.Modify();
        Commit();
        Other.Get(6); Other.Note := 'other'; Other.Modify();
        Commit();
    end;

    local procedure Write(var Row: Record "SR Row"): Text
    var Stored: Record "SR Row";
    begin
        Row.Qty := 5; Row.Modify();
        Stored.Get(6);
        exit(Stored.Note + '|' + Format(Stored.Qty));
    end;

    procedure Matrix(): Text
    var Row: Record "SR Row"; Other: Record "SR Row";
    begin
        Prepare(Row); Other.Get(6); Other.Modify();
        exit(Write(Row));
    end;

    procedure LockThenRollback(): Text
    var Row: Record "SR Row"; Lock: Record "SR Row";
    begin
        Prepare(Row); Lock.LockTable();
        asserterror Error('rollback');
        exit(Write(Row));
    end;

    procedure DatabaseErrorRollback(): Text
    var Row: Record "SR Row"; Dup: Record "SR Row";
    begin
        Prepare(Row);
        asserterror Dup.Get(999);
        exit(Write(Row));
    end;

    procedure RollbackThenOtherRow(): Text
    var Row: Record "SR Row"; Other: Record "SR Row";
    begin
        Prepare(Row);
        asserterror Error('rollback');
        Other.Get(8); Other.Modify();
        exit(Write(Row));
    end;

    procedure ModifyAllNoRow(): Text
    var Row: Record "SR Row"; All: Record "SR Row";
    begin
        Prepare(Row); All.SetRange("No.", 100); All.ModifyAll(Qty, 7);
        exit(Write(Row));
    end;

    procedure ModifyAllWithRow(): Text
    var Row: Record "SR Row"; All: Record "SR Row";
    begin
        Prepare(Row); All.ModifyAll(Qty, 7);
        exit(Write(Row));
    end;

    procedure TruncateOther(): Text
    var Row: Record "SR Row"; Other: Record "SR Other";
    begin
        Prepare(Row); Other."No." := 1; Other.Insert(); Other.Truncate();
        exit(Write(Row));
    end;
}
"#;

#[test]
fn row_contact_writes_the_stale_buffer() {
    // StaleRuleMatrix.al RowContact_CommitCount `base_2c`.
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Matrix")), "start|5");
}

#[test]
fn inputs_no_cell_measured_refuse_the_write() {
    for routine in [
        "LockThenRollback",
        "DatabaseErrorRollback",
        "RollbackThenOtherRow",
        "ModifyAllNoRow",
        "ModifyAllWithRow",
        "TruncateOther",
    ] {
        let r = al_testkit::run_in(SRC, "C", routine);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
    }
}
