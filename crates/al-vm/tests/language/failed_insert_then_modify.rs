//! `if not R.Insert() then R.Modify()` on a buffer read before, whose row another variable changed since, raises DB:RecordChanged
//! at the Modify, also with no Commit in between; a buffer never read, and one whose row nobody changed, write
//! (record/lt-y71-failed-insert-modify/Y71FailedInsertModify.al: FailedInsert_ThenModify_*, FailedInsertTwice_ThenModify).

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
    assert_eq!(text(al_testkit::run_in(SRC, "C", "ChangedElsewhere")), "DB:RecordChanged");
}

#[test]
fn a_row_nobody_changed_is_written() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "NothingChanged")), "5");
}

#[test]
fn a_buffer_never_read_is_written() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "FreshBuffer")), "5");
}
