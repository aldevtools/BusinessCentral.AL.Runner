//! `if not R.Insert() then R.Modify()` on a buffer read before, whose row another variable changed since, raises DB:RecordChanged
//! at the Modify, also with no Commit in between; a buffer never read, and one whose row nobody changed, write
//! (record/crud/Y71FailedInsertModify.al: FailedInsert_ThenModify_*, FailedInsertTwice_ThenModify; its seed is
//! committed, `Seed(true)` at :372, :389, :404, :419). With the seed not committed the table was written before the
//! read, and the Modify is written (al-runner issue 8721; corpus PR 1523,
//! record/crud/TestRecordFailedInsertMovedKey.al:784 R6, the same key changed by another variable after the read),
//! through the assumption `failed-insert-after-table-write`.

use al_testkit::text;

const SRC: &str = r#"
table 69720 "FIM Row" { fields { field(1; "No."; Code[10]) { } field(2; Qty; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
codeunit 69721 C
{
    local procedure Seed()
    var Row: Record "FIM Row";
    begin
        Row.DeleteAll();
        Row."No." := 'A';
        Row.Qty := 1;
        Row.Insert();
        Commit();
    end;

    procedure ChangedElsewhere(): Text
    var R: Record "FIM Row"; R2: Record "FIM Row";
    begin
        Seed();
        R.Get('A');
        R2.Get('A');
        R2.Qty := 2;
        R2.Modify();
        R.Qty := 5;
        if not R.Insert() then begin
            asserterror R.Modify();
            exit(GetLastErrorCode());
        end;
        exit('inserted');
    end;

    procedure ChangedElsewhereOpenSeed(): Text
    var R: Record "FIM Row"; R2: Record "FIM Row";
    begin
        R.DeleteAll();
        R."No." := 'A';
        R.Qty := 1;
        R.Insert();
        R.Get('A');
        R2.Get('A');
        R2.Qty := 2;
        R2.Modify();
        R.Qty := 5;
        if not R.Insert() then
            R.Modify();
        R2.Get('A');
        exit(Format(R2.Qty));
    end;

    procedure NothingChanged(): Text
    var R: Record "FIM Row";
    begin
        Seed();
        R.Get('A');
        R.Qty := 5;
        if not R.Insert() then
            R.Modify();
        R.Get('A');
        exit(Format(R.Qty));
    end;

    procedure FreshBuffer(): Text
    var R: Record "FIM Row"; R2: Record "FIM Row";
    begin
        Seed();
        R2.Get('A');
        R2.Qty := 2;
        R2.Modify();
        R."No." := 'A';
        R.Qty := 5;
        if not R.Insert() then
            R.Modify();
        R.Get('A');
        exit(Format(R.Qty));
    end;
}
"#;

#[test]
fn a_buffer_read_before_and_changed_elsewhere_raises() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "ChangedElsewhere")),
        "DB:RecordChanged"
    );
}

#[test]
fn the_same_key_changed_after_the_read_after_an_open_seed_is_written() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "ChangedElsewhereOpenSeed")), "5");
}

#[test]
fn a_row_nobody_changed_is_written() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "NothingChanged")), "5");
}

#[test]
fn a_buffer_never_read_is_written() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "FreshBuffer")), "5");
}
