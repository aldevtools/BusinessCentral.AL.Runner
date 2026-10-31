//! A stale buffer from the variable's own Insert, two Commits back (another variable wrote the row and committed in
//! between), is written when the transaction after the second Commit modified that same row through another variable
//! (al-runner issue 6522, the reporter's repro passed on real BC 29). On the assumption
//! `stale-own-insert-two-commits-row-written` until the corpus pin lands.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_testkit::text;

const SRC: &str = r#"
table 50380 "Stale Row" { fields { field(1; "No."; Code[20]) { } field(2; Description; Text[50]) { }
    field(3; "Remaining Quantity"; Decimal) { } } keys { key(PK; "No.") { } } }
codeunit 50380 Poster
{
    procedure PostAndCommit(No: Code[20]; Quantity: Decimal)
    begin
        Post(No, Quantity);
        Commit();
    end;

    procedure Post(No: Code[20]; Quantity: Decimal)
    var StaleRow: Record "Stale Row";
    begin
        StaleRow.Get(No);
        StaleRow."Remaining Quantity" -= Quantity;
        StaleRow.Modify();
    end;
}
codeunit 50390 C
{
    var Poster: Codeunit Poster;

    procedure RowWritten(): Text
    var StaleRow: Record "Stale Row"; Stored: Record "Stale Row";
    begin
        InsertStaleRow(StaleRow);
        Poster.Post(StaleRow."No.", 5);
        StaleRow.Validate(Description, 'Changed');
        StaleRow.Modify(true);
        Stored.Get('A');
        exit(Stored.Description + '|' + Format(Stored."Remaining Quantity"));
    end;

    procedure RowWrittenAndAnotherInserted(): Text
    var StaleRow: Record "Stale Row"; Another: Record "Stale Row";
    begin
        InsertStaleRow(StaleRow);
        Poster.Post(StaleRow."No.", 5);
        Another."No." := 'B'; Another.Insert();
        StaleRow.Validate(Description, 'Changed');
        StaleRow.Modify(true);
    end;

    procedure RowWrittenUnderLock(): Text
    var StaleRow: Record "Stale Row"; Locked: Record "Stale Row";
    begin
        InsertStaleRow(StaleRow);
        Locked.LockTable();
        Poster.Post(StaleRow."No.", 5);
        StaleRow.Validate(Description, 'Changed');
        StaleRow.Modify(true);
    end;

    local procedure InsertStaleRow(var StaleRow: Record "Stale Row")
    begin
        StaleRow.DeleteAll();
        StaleRow."No." := 'A';
        StaleRow.Description := 'Original';
        StaleRow."Remaining Quantity" := 100;
        StaleRow.Insert(true);
        Commit();
        Poster.PostAndCommit(StaleRow."No.", 10);
    end;
}
"#;

#[test]
fn the_stale_insert_copy_is_written_after_another_variable_modified_the_row() {
    clear_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "C", "RowWritten")), "Changed|100");
    assert!(take_hits().contains(&Assumption::StaleOwnInsertTwoCommitsRowWritten));
}

/// Other shapes stay refused until a test pins them.
#[test]
fn other_writes_or_a_lock_beside_the_modify_stay_refused() {
    for p in ["RowWrittenAndAnotherInserted", "RowWrittenUnderLock"] {
        let r = al_testkit::run_in(SRC, "C", p);
        assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{p}: {r:?}");
    }
}
