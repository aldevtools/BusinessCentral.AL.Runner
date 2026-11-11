//! A buffer made stale by its own write, then a rollback or a locking ReadIsolation read in the open transaction, then a write
//! through the buffer. Only the measured cells run; every other shape is refused as before corpus PR 1323 (review of runner
//! PR 7800).
//!
//! Measured after a rollback: after `asserterror Error('literal')`, nothing, or a Get and Modify of the row
//! (StaleRuleMatrix.al:206 Rollback); after a failed guarded `Codeunit.Run` that undid no write, the buffer's Modify() after
//! nothing or after one Get of the row (record/crud/CodeunitRunTableNoWrites.al:205, :267).
//! Measured with a locking read: the cells of `LOCKING_READ_CELLS` in al-vm `records/locking_read_cells.rs`
//! (CodeunitRunTableNoWrites.al:446-541, :629-740), and the four-Commit case of record/WbStaleUpdLockTests.al:16-25
//! (`stale_updlock_read.rs`).
//!
//! Each shape runs in its own codeunit: a refusal in one test of a codeunit stops the rest of it.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const OBJECTS: &str = r#"
table 50100 "P Row" { fields { field(1; "No."; Code[20]) { } field(2; Qty; Integer) { } field(3; Note; Text[30]) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 50101 "P Side" { fields { field(1; "No."; Code[20]) { } field(2; Qty; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
codeunit 50102 "P Cfg" { SingleInstance = true; var Mode: Text; procedure Set(M: Text) begin Mode := M; end; procedure Get(): Text begin exit(Mode); end; }
codeunit 50103 "P Fail"
{
    trigger OnRun()
    var Cfg: Codeunit "P Cfg"; R: Record "P Row";
    begin
        case Cfg.Get() of
            'modB': begin R.Get('B'); R.Qty += 1; R.Modify(); end;
            'modA': begin R.Get('A'); R.Qty += 1; R.Modify(); end;
            'commitmodB': begin Commit(); R.Get('B'); R.Qty += 1; R.Modify(); end;
        end;
        Error('the work failed');
    end;
}
codeunit 50104 "P Help"
{
    procedure MakeStale(var Row: Record "P Row")
    var Old: Record "P Row"; Other: Record "P Row"; S: Record "P Side";
    begin
        Old.DeleteAll(); S.DeleteAll();
        S."No." := 'S'; S.Insert();
        Old.Init(); Old."No." := 'B'; Old.Insert();
        Old.Init(); Old."No." := 'A'; Old.Insert();
        Commit();
        Row.Get('A'); Row.Qty := 1; Row.Modify();
        Commit();
        Other.Get('A'); Other.Qty := 5; Other.Modify();
        Commit();
    end;

    // The buffer of CodeunitRunTableNoWrites.al:555 Cell: Insert(true) and Modify(true), a partial record writes the row, one Commit.
    procedure MakeStaleOne(var Row: Record "P Row")
    var Old: Record "P Row"; Other: Record "P Row"; S: Record "P Side";
    begin
        Old.DeleteAll(); S.DeleteAll();
        S."No." := 'S'; S.Insert();
        Old.Init(); Old."No." := 'B'; Old.Insert();
        Commit();
        Row.Init(); Row."No." := 'A'; Row.Insert(true);
        Row.Qty := 1; Row.Modify(true);
        Other.SetLoadFields(Note); Other.Get('A'); Other.Note := 'other'; Other.Modify();
        Commit();
    end;

    // Another row of the table inserted and modified in the open transaction (CodeunitRunTableNoWrites.al:585-591).
    procedure OtherRow()
    var C: Record "P Row";
    begin
        C.Init(); C."No." := 'C'; C.Insert(true);
        C.Qty := 9; C.Modify(true);
    end;

    procedure FinishTrue(var Row: Record "P Row"; Expected: Text)
    begin
        Row.Qty := 2;
        asserterror begin
            Row.Modify(true);
            Error('WRITTEN');
        end;
        if (GetLastErrorCode() <> Expected) and (GetLastErrorText() <> Expected) then
            Error('expected %1, got %2: %3', Expected, GetLastErrorCode(), GetLastErrorText());
    end;

    procedure FinishDelete(var Row: Record "P Row"; Expected: Text)
    begin
        asserterror begin
            Row.Delete();
            Error('DELETED');
        end;
        if (GetLastErrorCode() <> Expected) and (GetLastErrorText() <> Expected) then
            Error('expected %1, got %2: %3', Expected, GetLastErrorCode(), GetLastErrorText());
    end;

    procedure Finish(var Row: Record "P Row"; Expected: Text)
    begin
        Row.Note := 'mine';
        asserterror begin
            Row.Modify();
            Error('WRITTEN');
        end;
        if (GetLastErrorCode() <> Expected) and (GetLastErrorText() <> Expected) then
            Error('expected %1, got %2: %3', Expected, GetLastErrorCode(), GetLastErrorText());
    end;
}
"#;

/// Compiles one codeunit per shape and gives each shape's outcome.
fn outcomes(shapes: &[(&str, &str)]) -> Vec<(String, Outcome)> {
    let mut src = OBJECTS.to_string();
    for (i, (name, body)) in shapes.iter().enumerate() {
        // A body that makes the buffer stale itself names the helper.
        let stale = if body.contains("H.MakeStaleOne(Row);") {
            ""
        } else {
            "H.MakeStale(Row);"
        };
        src.push_str(&format!(
            "codeunit {} \"{name}\"\n{{\n    Subtype = Test;\n    TestPermissions = Disabled;\n    [Test] procedure {name}()\n    var Row: Record \"P Row\"; Check: Record \"P Row\"; R: Record \"P Row\"; S: Record \"P Side\"; H: Codeunit \"P Help\"; Cfg: Codeunit \"P Cfg\";\n    begin\n        {stale}\n        {body}\n    end;\n}}\n",
            50200 + i
        ));
    }
    let mut all = Vec::new();
    for (name, _) in shapes {
        all.extend(outcomes_in(&src, name));
    }
    all
}

fn check(shapes: &[(&str, &str, &str)]) {
    let cells: Vec<(&str, &str)> = shapes.iter().map(|(n, b, _)| (*n, *b)).collect();
    let got = outcomes(&cells);
    let mut wrong = Vec::new();
    for ((name, _, want), (_, outcome)) in shapes.iter().zip(&got) {
        let ok = match *want {
            "refused" => matches!(outcome, Outcome::NotRun(_)),
            _ => matches!(outcome, Outcome::Pass),
        };
        if !ok {
            wrong.push(format!("{name}: want {want}, got {outcome:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_rollback_kind_decides_what_runs_after_it() {
    check(&[
        // asserterror: nothing after it, or a Get and Modify of the row (StaleRuleMatrix.al:206 rm_rb_*, rm_rb_rm_2c).
        (
            "AssertNothing",
            "asserterror Error('x'); H.Finish(Row, 'DB:RecordChanged');",
            "pass",
        ),
        (
            "AssertGetModify",
            "asserterror Error('x'); Check.Get('A'); Check.Qty := 9; Check.Modify(); H.Finish(Row, 'WRITTEN');",
            "pass",
        ),
        // A Get of the row alone is measured after a guarded run only.
        (
            "AssertGetAlone",
            "asserterror Error('x'); Check.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        // A failed guarded run that undid nothing: nothing, or a Get of the row alone (CodeunitRunTableNoWrites.al:205, :267).
        (
            "GuardedNothing",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; H.Finish(Row, 'DB:RecordChanged');",
            "pass",
        ),
        (
            "GuardedGetAlone",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; Check.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "pass",
        ),
        // A Get and Modify of the row is measured after an asserterror only.
        (
            "GuardedGetModify",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; Check.Get('A'); Check.Qty := 9; Check.Modify(); H.Finish(Row, 'WRITTEN');",
            "refused",
        ),
        // A guarded run whose rollback undid a write is not measured.
        (
            "GuardedUndidOtherRow",
            "Cfg.Set('modB'); if Codeunit.Run(Codeunit::\"P Fail\") then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedUndidRow",
            "Cfg.Set('modA'); if Codeunit.Run(Codeunit::\"P Fail\") then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedCommitUndidOtherRow",
            "Cfg.Set('commitmodB'); if Codeunit.Run(Codeunit::\"P Fail\") then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        // After a guarded run only the buffer's Modify() is measured: a Delete and Modify(true) are not (round 3 of the review).
        (
            "GuardedThenDelete",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; H.FinishDelete(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedGetThenDelete",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; Check.Get('A'); H.FinishDelete(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedGetThenModifyTrue",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; Check.Get('A'); H.FinishTrue(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedTwoGets",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; Check.Get('A'); Check.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        // After an asserterror a Delete follows the rule as before the pin (the binary of main, ca0c74808).
        (
            "AssertThenDelete",
            "asserterror Error('x'); H.FinishDelete(Row, 'DB:RecordChanged');",
            "pass",
        ),
        // Both kinds in one transaction are not measured.
        (
            "AssertThenGuarded",
            "asserterror Error('x'); Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; Check.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
    ]);
}

#[test]
fn only_the_pinned_locking_read_cells_run() {
    check(&[
        // The pinned cells, two Commits after the own write, Modify(): one locking read of another row, nothing else
        // (CodeunitRunTableNoWrites.al:446, :457, :468, :512, :523, :534).
        (
            "TwoCommitsUpdLockGetOtherRow",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.Get('B'); H.Finish(Row, 'DB:RecordChanged');",
            "pass",
        ),
        (
            "TwoCommitsUpdLockFindSetOtherRow",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.SetRange(\"No.\", 'B'); R.FindSet(); H.Finish(Row, 'DB:RecordChanged');",
            "pass",
        ),
        (
            "TwoCommitsRepeatableReadGetOtherRow",
            "R.ReadIsolation := IsolationLevel::RepeatableRead; R.Get('B'); H.Finish(Row, 'DB:RecordChanged');",
            "pass",
        ),
        // The pinned cells, one Commit after the own Insert(true) and Modify(true), Modify(true): one locking read, beside nothing
        // (DB:RecordChanged) or after another row was inserted and modified (written) (:629, :642, :654, :680, :692, :705, :730).
        (
            "OneCommitUpdLockFindFirstOtherTableOtherRow",
            "H.MakeStaleOne(Row); H.OtherRow(); S.ReadIsolation := IsolationLevel::UpdLock; if S.FindFirst() then; H.FinishTrue(Row, 'WRITTEN');",
            "pass",
        ),
        (
            "OneCommitUpdLockFindFirstOtherTable",
            "H.MakeStaleOne(Row); S.ReadIsolation := IsolationLevel::UpdLock; if S.FindFirst() then; H.FinishTrue(Row, 'DB:RecordChanged');",
            "pass",
        ),
        (
            "OneCommitUpdLockGetOwnRowOtherRow",
            "H.MakeStaleOne(Row); H.OtherRow(); R.ReadIsolation := IsolationLevel::UpdLock; R.Get('A'); H.FinishTrue(Row, 'WRITTEN');",
            "pass",
        ),
        (
            "OneCommitUpdLockGetOwnRow",
            "H.MakeStaleOne(Row); R.ReadIsolation := IsolationLevel::UpdLock; R.Get('A'); H.FinishTrue(Row, 'DB:RecordChanged');",
            "pass",
        ),
        (
            "OneCommitUpdLockGetOtherRowOtherRow",
            "H.MakeStaleOne(Row); H.OtherRow(); R.ReadIsolation := IsolationLevel::UpdLock; R.Get('B'); H.FinishTrue(Row, 'WRITTEN');",
            "pass",
        ),
        (
            "OneCommitUpdLockGetOtherRow",
            "H.MakeStaleOne(Row); R.ReadIsolation := IsolationLevel::UpdLock; R.Get('B'); H.FinishTrue(Row, 'DB:RecordChanged');",
            "pass",
        ),
        (
            "OneCommitRepeatableReadGetOwnRowOtherRow",
            "H.MakeStaleOne(Row); H.OtherRow(); R.ReadIsolation := IsolationLevel::RepeatableRead; R.Get('A'); H.FinishTrue(Row, 'WRITTEN');",
            "pass",
        ),
        // Round 3 of the review: combinations beside the pinned cells.
        // A Get of a missing row.
        (
            "UpdLockGetMissing",
            "R.ReadIsolation := IsolationLevel::UpdLock; if R.Get('ZZ') then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        // Two locking reads.
        (
            "UpdLockGetOwnPlusFindFirstOtherTable",
            "H.MakeStaleOne(Row); R.ReadIsolation := IsolationLevel::UpdLock; R.Get('A'); S.ReadIsolation := IsolationLevel::UpdLock; if S.FindFirst() then; H.FinishTrue(Row, 'DB:RecordChanged');",
            "refused",
        ),
        // A FindSet without the filter on the other row.
        (
            "UpdLockFindSetUnfiltered",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.FindSet(); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "UpdLockFindSetOtherFilter",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.SetFilter(\"No.\", '>=%1', 'B'); R.FindSet(); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        // A Delete through the stale buffer: the cells pin Modify only.
        (
            "UpdLockGetOtherRowThenDelete",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.Get('B'); H.FinishDelete(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "UpdLockModifyOtherRowThenDelete",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.Get('B'); R.Qty := 4; R.Modify(); H.FinishDelete(Row, 'DELETED');",
            "refused",
        ),
        // The other Modify form, the other Commit count, the other isolation level or read of a cell.
        (
            "TwoCommitsModifyTrue",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.Get('B'); H.FinishTrue(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "TwoCommitsUpdLockGetOwnRow",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "TwoCommitsRepeatableReadFindSet",
            "R.ReadIsolation := IsolationLevel::RepeatableRead; R.SetRange(\"No.\", 'B'); R.FindSet(); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "TwoCommitsUpdLockFindFirstOtherTable",
            "S.ReadIsolation := IsolationLevel::UpdLock; if S.FindFirst() then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "OneCommitModifyFalse",
            "H.MakeStaleOne(Row); R.ReadIsolation := IsolationLevel::UpdLock; R.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "OneCommitRepeatableReadGetOwnRow",
            "H.MakeStaleOne(Row); R.ReadIsolation := IsolationLevel::RepeatableRead; R.Get('A'); H.FinishTrue(Row, 'DB:RecordChanged');",
            "refused",
        ),
        // Another read or lock beside the one locking read.
        (
            "UpdLockGetPlusFindSetTrueOtherTable",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.Get('B'); S.FindSet(true); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "PlainGetPlusUpdLockGet",
            "Check.Get('B'); R.ReadIsolation := IsolationLevel::UpdLock; R.Get('B'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "UpdLockFindSetThenNext",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.SetRange(\"No.\", 'B'); R.FindSet(); if R.Next() = 0 then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        // Other writes beside it than the Insert and Modify of one other row, before the read.
        (
            "ModifyOtherRowThenUpdLockGetOwn",
            "H.MakeStaleOne(Row); Check.Get('B'); Check.Qty := 3; Check.Modify(); R.ReadIsolation := IsolationLevel::UpdLock; R.Get('A'); H.FinishTrue(Row, 'WRITTEN');",
            "refused",
        ),
        (
            "UpdLockGetThenModifyOtherRow",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.Get('B'); R.Qty := 4; R.Modify(); H.Finish(Row, 'WRITTEN');",
            "refused",
        ),
        (
            "UpdLockGetThenOtherRow",
            "H.MakeStaleOne(Row); R.ReadIsolation := IsolationLevel::UpdLock; R.Get('A'); H.OtherRow(); H.FinishTrue(Row, 'WRITTEN');",
            "refused",
        ),
        (
            "UpdLockGetOfTheInsertedRow",
            "H.MakeStaleOne(Row); H.OtherRow(); R.ReadIsolation := IsolationLevel::UpdLock; R.Get('C'); H.FinishTrue(Row, 'WRITTEN');",
            "refused",
        ),
        // Not measured before round 3 either: another read form, a LockTable beside it, a write of another table, row contact.
        (
            "UpdLockFindEq",
            "R.\"No.\" := 'A'; R.ReadIsolation := IsolationLevel::UpdLock; R.Find(); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "UpdLockCount",
            "R.ReadIsolation := IsolationLevel::UpdLock; if R.Count() = 0 then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "RepeatableReadFindLast",
            "R.ReadIsolation := IsolationLevel::RepeatableRead; R.FindLast(); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "UpdLockOtherTableWritten",
            "S.ReadIsolation := IsolationLevel::UpdLock; S.Get('S'); S.Qty := 4; S.Modify(); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "LockTableOtherThenUpdLock",
            "S.LockTable(); R.ReadIsolation := IsolationLevel::UpdLock; R.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "UpdLockThenLockTableOwn",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.Get('A'); R.LockTable(); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "UpdLockThenRowModified",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.Get('A'); R.Qty := 8; R.Modify(); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "UpdLockAfterAssertError",
            "asserterror Error('x'); R.ReadIsolation := IsolationLevel::UpdLock; R.Get('B'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "UpdLockAssignedBeforeCommit",
            "R.ReadIsolation := IsolationLevel::UpdLock; Commit(); R.Get('B'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
    ]);
}
