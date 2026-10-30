//! A stale buffer from the variable's own Insert and Modify, in the three shapes of al-runner issue 6028 (the reporter's
//! repro on real BC 28.5, registered assumption `stale-own-insert-lock-other-or-other-row-write`):
//! - two Commits later, the new transaction runs LockTable on another table, inserts into it and reads the table by
//!   FindSet: the stale Modify raises DB:RecordChanged;
//! - three Commits later, the new transaction Modifies another row of the table: the stale Modify writes the whole buffer;
//! - three Commits later, the new transaction Deletes another row of the table: the stale Delete deletes the row.
//!
//! Every neighbour nobody measured is refused.

use al_builtins::assumptions::{clear_hits, take_hits};
use al_builtins::native::Signal;

const SRC: &str = r#"
table 50160 "Repro N2 Row" { fields { field(1; "No."; Code[20]) { } field(2; Note; Text[30]) { } field(3; Qty; Integer) { } }
    keys { key(PK; "No.") { } } }
table 50161 "Repro N2 Other" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 50162 "Repro N2 Third" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
codeunit 50160 C
{
    local procedure PrepareStaleBuffer(var Row: Record "Repro N2 Row")
    var Other: Record "Repro N2 Row"; Another: Record "Repro N2 Row";
    begin
        Commit();
        Row."No." := 'N2'; Row.Note := 'start'; Row.Insert(true); Row.Qty := 1; Row.Modify(true);
        Another."No." := 'N2-OTHER'; Another.Insert(true);
        Other.Get('N2'); Other.Note := 'other'; Other.Modify(false);
    end;

    local procedure Prepare(var Row: Record "Repro N2 Row"; Commits: Integer)
    var i: Integer; Middle: Record "Repro N2 Other";
    begin
        PrepareStaleBuffer(Row);
        Commit();
        for i := 2 to Commits do begin
            Middle."No." := 'MIDDLE-' + Format(i); Middle.Insert(true);
            Commit();
        end;
    end;

    local procedure LockAnotherTableAndRead(Read: Boolean)
    var Middle: Record "Repro N2 Other"; Reader: Record "Repro N2 Row";
    begin
        Middle.LockTable();
        Middle."No." := 'LOCKED'; Middle.Insert(false);
        if Read then
            Reader.FindSet();
    end;

    local procedure TryStaleModify(var Row: Record "Repro N2 Row"): Text
    var Stored: Record "Repro N2 Row";
    begin
        Row.Qty := 5;
        asserterror Row.Modify(true);
        exit(GetLastErrorCode());
    end;

    local procedure StaleModify(var Row: Record "Repro N2 Row"): Text
    var Stored: Record "Repro N2 Row";
    begin
        Row.Qty := 5; Row.Modify(true);
        Stored.Get('N2');
        exit(Stored.Note + '|' + Format(Stored.Qty));
    end;

    local procedure StaleDelete(var Row: Record "Repro N2 Row"): Text
    var Stored: Record "Repro N2 Row";
    begin
        Row.Delete(true);
        exit(Format(Stored.Get('N2')));
    end;

    local procedure ModifyAnother()
    var Another: Record "Repro N2 Row";
    begin
        Another.Get('N2-OTHER'); Another.Qty := 7; Another.Modify(false);
    end;

    local procedure DeleteAnother()
    var Another: Record "Repro N2 Row";
    begin
        Another.Get('N2-OTHER'); Another.Delete(false);
    end;

    // Shape 1.
    procedure LockTwo(): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, 2); LockAnotherTableAndRead(true);
        exit(TryStaleModify(Row));
    end;

    // Shape 2.
    procedure ModifyThree(): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, 3); ModifyAnother();
        exit(StaleModify(Row));
    end;

    // Shape 3.
    procedure DeleteThree(): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, 3); DeleteAnother();
        exit(StaleDelete(Row));
    end;

    // Neighbours nobody measured.
    procedure LockTwoNoRead(): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, 2); LockAnotherTableAndRead(false);
        exit(TryStaleModify(Row));
    end;

    procedure LockTwoStaleDelete(): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, 2); LockAnotherTableAndRead(true);
        exit(StaleDelete(Row));
    end;

    procedure LockOne(): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, 1); LockAnotherTableAndRead(true);
        exit(TryStaleModify(Row));
    end;

    procedure LockThree(): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, 3); LockAnotherTableAndRead(true);
        exit(TryStaleModify(Row));
    end;

    procedure LockTwoAndThirdTable(): Text
    var Row: Record "Repro N2 Row"; Third: Record "Repro N2 Third";
    begin
        Prepare(Row, 2); LockAnotherTableAndRead(true);
        Third."No." := 'T'; Third.Insert(false);
        exit(TryStaleModify(Row));
    end;

    procedure LockTwoUpdLock(): Text
    var Row: Record "Repro N2 Row"; Probe: Record "Repro N2 Third";
    begin
        Prepare(Row, 2); LockAnotherTableAndRead(true);
        Probe.ReadIsolation := IsolationLevel::UpdLock;
        if Probe.FindFirst() then;
        exit(TryStaleModify(Row));
    end;

    procedure ModifyTwo(): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, 2); ModifyAnother();
        exit(StaleModify(Row));
    end;

    procedure ModifyFour(): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, 4); ModifyAnother();
        exit(StaleModify(Row));
    end;

    procedure ModifyThreeStaleDelete(): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, 3); ModifyAnother();
        exit(StaleDelete(Row));
    end;

    procedure DeleteThreeStaleModify(): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, 3); DeleteAnother();
        exit(StaleModify(Row));
    end;

    procedure ModifyThreeAndInsert(): Text
    var Row: Record "Repro N2 Row"; Third: Record "Repro N2 Row";
    begin
        Prepare(Row, 3); ModifyAnother();
        Third."No." := 'N2-THIRD'; Third.Insert(false);
        exit(StaleModify(Row));
    end;

    procedure ModifyThreeAndOtherTable(): Text
    var Row: Record "Repro N2 Row"; Third: Record "Repro N2 Third";
    begin
        Prepare(Row, 3); ModifyAnother();
        Third."No." := 'T'; Third.Insert(false);
        exit(StaleModify(Row));
    end;

    procedure ModifyAllThree(): Text
    var Row: Record "Repro N2 Row"; Another: Record "Repro N2 Row";
    begin
        Prepare(Row, 3);
        Another.SetRange("No.", 'N2-OTHER'); Another.ModifyAll(Qty, 7);
        exit(StaleModify(Row));
    end;

    procedure RenameThree(): Text
    var Row: Record "Repro N2 Row"; Another: Record "Repro N2 Row";
    begin
        Prepare(Row, 3);
        Another.Get('N2-OTHER'); Another.Rename('N2-RENAMED');
        exit(StaleModify(Row));
    end;

    procedure InsertThree(): Text
    var Row: Record "Repro N2 Row"; Another: Record "Repro N2 Row";
    begin
        Prepare(Row, 3);
        Another."No." := 'N2-THIRD'; Another.Insert(true);
        exit(StaleModify(Row));
    end;

    procedure ModifyThreeLocked(): Text
    var Row: Record "Repro N2 Row"; Locked: Record "Repro N2 Other";
    begin
        Prepare(Row, 3); Locked.LockTable(); ModifyAnother();
        exit(StaleModify(Row));
    end;
    // Writer TH (al-runner issue 6040) and writer TE: the same neighbours with 1 to 10 Commits
    // (record/lt-sb-stale-own-modify/SbStaleOwnModify.al TE_*, TH_* on branch al-runtime-rs-staging-pub-th).
    procedure LockMany(Commits: Integer; Deleting: Boolean): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, Commits); LockAnotherTableAndRead(true);
        Row.Qty := 5;
        if Deleting then asserterror Row.Delete(false) else asserterror Row.Modify(true);
        exit(GetLastErrorCode());
    end;

    procedure LockOnly(Commits: Integer): Text
    var Row: Record "Repro N2 Row"; Middle: Record "Repro N2 Other";
    begin
        Prepare(Row, Commits); Middle.LockTable();
        exit(TryStaleModify(Row));
    end;

    procedure LockSame(Commits: Integer): Text
    var Row: Record "Repro N2 Row"; Same: Record "Repro N2 Row";
    begin
        Prepare(Row, Commits); Same.LockTable();
        exit(StaleModify(Row));
    end;

    procedure ModifyMany(Commits: Integer; Deleting: Boolean): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, Commits); ModifyAnother();
        if Deleting then exit(StaleDelete(Row));
        exit(StaleModify(Row));
    end;

    procedure DeleteMany(Commits: Integer; Deleting: Boolean): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, Commits); DeleteAnother();
        if Deleting then exit(StaleDelete(Row));
        exit(StaleModify(Row));
    end;

    procedure InsertMany(Commits: Integer): Text
    var Row: Record "Repro N2 Row"; Another: Record "Repro N2 Row";
    begin
        Prepare(Row, Commits);
        Another."No." := 'N2-THIRD'; Another.Insert(true);
        exit(StaleModify(Row));
    end;

    procedure NothingMany(Commits: Integer): Text
    var Row: Record "Repro N2 Row";
    begin
        Prepare(Row, Commits);
        exit(TryStaleModify(Row));
    end;

    // The reporter's shape 1: an Insert into a table that is not locked beside the lock on another table.
    procedure LockEightWithLog(): Text
    var Row: Record "Repro N2 Row"; Third: Record "Repro N2 Third";
    begin
        Prepare(Row, 8);
        Third."No." := 'LOG-1'; Third.Insert(true);
        LockAnotherTableAndRead(true);
        Third."No." := 'LOG-2'; Third.Insert(true);
        exit(TryStaleModify(Row));
    end;
}
"#;

#[test]
fn a_lock_on_another_table_two_commits_later_raises_record_changed() {
    clear_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "C", "LockTwo")), "DB:RecordChanged");
    assert!(take_hits().is_empty());
}

#[test]
fn a_modify_of_another_row_three_commits_later_writes_the_whole_buffer() {
    clear_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "C", "ModifyThree")), "start|5");
    assert!(take_hits().is_empty());
}

#[test]
fn a_delete_of_another_row_three_commits_later_lets_the_stale_delete_delete() {
    clear_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "C", "DeleteThree")), "No");
    assert!(take_hits().is_empty());
}

#[test]
fn the_unmeasured_neighbours_are_refused() {
    for routine in [
        "LockTwoNoRead",
        "LockTwoUpdLock",
        "ModifyThreeAndInsert",
        "ModifyThreeAndOtherTable",
        "ModifyAllThree",
        "ModifyThreeLocked",
    ] {
        let r = al_testkit::run_in(SRC, "C", routine);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
    }
}

#[test]
fn the_neighbours_writers_te_and_th_measured_follow_one_rule_at_any_commit_count() {
    let run = |call: &str| {
        let src = SRC.replacen(
            "codeunit 50160 C\n{",
            &format!("codeunit 50160 C\n{{\n    procedure Probe(): Text begin exit({call}); end;"),
            1,
        );
        // Pinned at every count (SbStaleOwnModify.al Y2_4C_*, Y2_5C_*, Y2_10C_*, Y2_12C_*): no assumption.
        text(al_testkit::run_in(&src, "C", "Probe"))
    };
    // The three shapes of 6028 (two Commits, lock; three Commits, Modify or Delete of another row) keep their own assumption.
    for n in [1, 3, 4, 5, 10] {
        assert_eq!(run(&format!("LockMany({n}, false)")), "DB:RecordChanged", "{n}");
    }
    for n in [1, 2, 3, 4, 5, 10] {
        assert_eq!(run(&format!("LockMany({n}, true)")), "DB:RecordChanged", "{n}");
        assert_eq!(run(&format!("LockSame({n})")), "start|5", "{n}");
    }
    for n in [1, 2, 3] {
        assert_eq!(run(&format!("LockOnly({n})")), "DB:RecordChanged", "{n}");
    }
    for n in [1, 2, 4, 5, 10] {
        assert_eq!(run(&format!("ModifyMany({n}, false)")), "start|5", "{n}");
        assert_eq!(run(&format!("DeleteMany({n}, false)")), "start|5", "{n}");
    }
    assert_eq!(run("DeleteMany(3, false)"), "start|5");
    assert_eq!(run("ModifyMany(3, true)"), "No");
    for n in [4, 5, 10] {
        assert_eq!(run(&format!("ModifyMany({n}, true)")), "No", "{n}");
        assert_eq!(run(&format!("DeleteMany({n}, true)")), "No", "{n}");
    }
    assert_eq!(run("InsertMany(3)"), "start|5");
    assert_eq!(run("RenameThree()"), "start|5");
    assert_eq!(run("LockEightWithLog()"), "DB:RecordChanged");
}

#[test]
fn a_stale_buffer_with_nothing_in_the_new_transaction_is_refused_at_any_commit_count() {
    for n in [1, 4, 5, 10] {
        let src = SRC.replacen(
            "codeunit 50160 C\n{",
            &format!("codeunit 50160 C\n{{\n    procedure Probe(): Text begin exit(NothingMany({n})); end;"),
            1,
        );
        assert_eq!(text(al_testkit::run_in(&src, "C", "Probe")), "DB:RecordChanged", "{n}");
    }
}

use al_testkit::text;
