//! A stale buffer from the variable's own Insert and Modify, made stale before two Commits, is written whole once the
//! transaction after the second Commit inserted another row of the table (al-runner issue 5954, the reporter's repro on
//! real BC; registered assumption `stale-own-insert-two-commits-other-insert`). Without that Insert BC refuses it.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};

const SRC: &str = r#"
table 50150 "Repro Row" { fields { field(1; "No."; Code[20]) { } field(2; Note; Text[30]) { } field(3; Qty; Integer) { } }
    keys { key(PK; "No.") { } } }
table 50151 "Repro Other" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
codeunit 50150 C
{
    local procedure Prepare(var Row: Record "Repro Row")
    var Other: Record "Repro Row"; Middle: Record "Repro Other";
    begin
        Row."No." := 'R2'; Row.Note := 'start'; Row.Insert(true); Row.Qty := 1; Row.Modify(true);
        Other.Get('R2'); Other.Note := 'other'; Other.Modify(false);
        Commit();
        Middle."No." := 'MIDDLE'; Middle.Insert(true);
        Commit();
    end;

    procedure OneCommit(): Text
    var Row: Record "Repro Row"; Other: Record "Repro Row"; Another: Record "Repro Row"; Stored: Record "Repro Row";
    begin
        Row."No." := 'R2'; Row.Note := 'start'; Row.Insert(true); Row.Qty := 1; Row.Modify(true);
        Other.Get('R2'); Other.Note := 'other'; Other.Modify(false);
        Commit();
        Another."No." := 'R2-OTHER'; Another.Insert(true);
        Row.Qty := 5; Row.Modify(true);
        Stored.Get('R2');
        exit(Stored.Note + '|' + Format(Stored.Qty));
    end;

    procedure Written(): Text
    var Row: Record "Repro Row"; Another: Record "Repro Row"; Stored: Record "Repro Row";
    begin
        Prepare(Row);
        Another."No." := 'R2-OTHER'; Another.Insert(true);
        Row.Qty := 5; Row.Modify(true);
        Stored.Get('R2');
        exit(Stored.Note + '|' + Format(Stored.Qty));
    end;
}
"#;

#[test]
fn the_stale_buffer_is_written_after_an_insert_of_another_row() {
    clear_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Written")), "start|5");
    assert!(take_hits().contains(&Assumption::StaleOwnInsertTwoCommitsOtherInsert));
}

#[test]
fn one_commit_and_an_insert_of_another_row_also_writes() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "OneCommit")), "start|5");
}

use al_testkit::text;
