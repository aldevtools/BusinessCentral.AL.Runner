//! A buffer made stale by its own write, then a rollback or a locking ReadIsolation read in the open transaction, then a write
//! through the buffer. Only the measured cells run; every other shape is refused as before corpus PR 1323 (review of runner
//! PR 7800).
//!
//! Measured after a rollback: after `asserterror Error('literal')`, nothing, or a Get and Modify of the row
//! (StaleRuleMatrix.al:206 Rollback); after a failed guarded `Codeunit.Run` whose body is the pinned one (Rec.Modify(), Commit(),
//! Error('literal') on the record passed to the static call), the buffer's Modify() after nothing or after one Get of the row (record/crud/CodeunitRunTableNoWrites.al:205, :267).
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
    var Cfg: Codeunit "P Cfg"; R: Record "P Row"; S: Record "P Side"; T: Record "P Row" temporary;
    begin
        case Cfg.Get() of
            'temp': begin T."No." := 'T'; T.Insert(); end;
            'subst': Error('x %1', 5);
            'empty': Error('');
            'readA': R.Get('A');
            'commitread': begin Commit(); R.Get('A'); end;
            'inner': if Codeunit.Run(Codeunit::"P Fail2") then;
            'sidecommit': begin S.Get('S'); S.Qty += 1; S.Modify(); Commit(); end;
            'modB': begin R.Get('B'); R.Qty += 1; R.Modify(); end;
            'modA': begin R.Get('A'); R.Qty += 1; R.Modify(); end;
            'commitmodB': begin Commit(); R.Get('B'); R.Qty += 1; R.Modify(); end;
        end;
        Error('the work failed');
    end;
}
codeunit 50106 "P Fail2" { trigger OnRun() begin Error('inner failed'); end; }
// The run of record/crud/CodeunitRunTableNoWrites.al:69 (CRT Runner): Rec changed, Rec.Modify(), Commit(), Error('literal'). A
// mode adds one step the pinned run does not have.
codeunit 50107 "P Run"
{
    TableNo = "P Row";
    trigger OnRun()
    var Cfg: Codeunit "P Cfg"; R: Record "P Row"; S: Record "P Side"; T: Record "P Row" temporary;
    begin
        case Cfg.Get() of
            'read': R.Get('B');
            'temp': begin T."No." := 'T'; T.Insert(); end;
            'inner': if Codeunit.Run(Codeunit::"P Fail2") then;
        end;
        Rec.Qty := 3;
        if Cfg.Get() = 'true' then Rec.Modify(true) else Rec.Modify();
        if Cfg.Get() = 'side' then begin S.Get('S'); S.Qty += 1; S.Modify(); end;
        if Cfg.Get() <> 'nocommit' then Commit();
        if Cfg.Get() = 'twocommits' then Commit();
        if Cfg.Get() = 'readafter' then R.Get('B');
        if Cfg.Get() = 'empty' then Error('');
        Error('the work failed');
    end;
}
codeunit 50104 "P Help"
{
    // The seed of CodeunitRunTableNoWrites.al:103: the caller's buffer holds the stored row.
    procedure Seed(var Row: Record "P Row")
    var Old: Record "P Row"; S: Record "P Side";
    begin
        Old.DeleteAll(); S.DeleteAll();
        S."No." := 'S'; S.Insert();
        Old.Init(); Old."No." := 'B'; Old.Insert();
        Row.Init(); Row."No." := 'A'; Row.Insert();
        Commit();
        Row.Get('A'); Row.Qty := 1; Row.Modify();
        Commit();
    end;

    [TryFunction] procedure TryFail() begin Error('t'); end;

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
        let stale = if body.contains("H.MakeStaleOne(Row);") || body.contains("H.Seed(Row);") {
            ""
        } else {
            "H.MakeStale(Row);"
        };
        src.push_str(&format!(
            "codeunit {} \"{name}\"\n{{\n    Subtype = Test;\n    TestPermissions = Disabled;\n    [Test] procedure {name}()\n    var Row: Record \"P Row\"; Check: Record \"P Row\"; R: Record \"P Row\"; S: Record \"P Side\"; H: Codeunit \"P Help\"; Cfg: Codeunit \"P Cfg\"; F: Codeunit \"P Fail\"; FR: Codeunit \"P Run\";\n    begin\n        {stale}\n        {body}\n    end;\n}}\n",
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
        // A failed guarded run with the pinned body (Rec.Modify(), Commit(), Error('literal') on the record passed to a static
        // Codeunit.Run): the buffer's Modify() after nothing, or after one Get of the row (CodeunitRunTableNoWrites.al:205, :267).
        (
            "GuardedPinned",
            "H.Seed(Row); Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Run\", Row) then; H.Finish(Row, 'DB:RecordChanged');",
            "pass",
        ),
        (
            "GuardedPinnedGet",
            "H.Seed(Row); Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Run\", Row) then; Check.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "pass",
        ),
        // The run without the Commit undid its Modify: the buffer is not stale and the Modify is written (:280).
        (
            "GuardedPinnedNoCommit",
            "H.Seed(Row); Cfg.Set('nocommit'); if Codeunit.Run(Codeunit::\"P Run\", Row) then; H.Finish(Row, 'WRITTEN');",
            "pass",
        ),
        // A Get and Modify of the row is measured after an asserterror only.
        (
            "GuardedGetModify",
            "H.Seed(Row); Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Run\", Row) then; Check.Get('A'); Check.Qty := 9; Check.Modify(); H.Finish(Row, 'WRITTEN');",
            "refused",
        ),
        // The pinned body with one more step, or run another way, is not measured (round 4 of the review).
        (
            "GuardedBody_read",
            "H.Seed(Row); Cfg.Set('read'); if Codeunit.Run(Codeunit::\"P Run\", Row) then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedBody_temp",
            "H.Seed(Row); Cfg.Set('temp'); if Codeunit.Run(Codeunit::\"P Run\", Row) then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedBody_inner",
            "H.Seed(Row); Cfg.Set('inner'); if Codeunit.Run(Codeunit::\"P Run\", Row) then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedBody_true",
            "H.Seed(Row); Cfg.Set('true'); if Codeunit.Run(Codeunit::\"P Run\", Row) then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedBody_side",
            "H.Seed(Row); Cfg.Set('side'); if Codeunit.Run(Codeunit::\"P Run\", Row) then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedBody_twocommits",
            "H.Seed(Row); Cfg.Set('twocommits'); if Codeunit.Run(Codeunit::\"P Run\", Row) then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedBody_readafter",
            "H.Seed(Row); Cfg.Set('readafter'); if Codeunit.Run(Codeunit::\"P Run\", Row) then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedBody_empty",
            "H.Seed(Row); Cfg.Set('empty'); if Codeunit.Run(Codeunit::\"P Run\", Row) then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedBodyInstanceForm",
            "H.Seed(Row); Cfg.Set(''); if FR.Run(Row) then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        // A run that is not the pinned body: an Error alone, or with another step before it (the reviewer's scenarios; all refused
        // by the binary of main, ca0c74808).
        (
            "GuardedNothing",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedGetAlone",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; Check.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedOther_temp",
            "Cfg.Set('temp'); if Codeunit.Run(Codeunit::\"P Fail\") then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedOther_subst",
            "Cfg.Set('subst'); if Codeunit.Run(Codeunit::\"P Fail\") then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedOther_empty",
            "Cfg.Set('empty'); if Codeunit.Run(Codeunit::\"P Fail\") then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedOther_readA",
            "Cfg.Set('readA'); if Codeunit.Run(Codeunit::\"P Fail\") then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedOther_commitread",
            "Cfg.Set('commitread'); if Codeunit.Run(Codeunit::\"P Fail\") then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedOther_inner",
            "Cfg.Set('inner'); if Codeunit.Run(Codeunit::\"P Fail\") then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedOther_sidecommit",
            "Cfg.Set('sidecommit'); if Codeunit.Run(Codeunit::\"P Fail\") then; H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedOtherTwice",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; if Codeunit.Run(Codeunit::\"P Fail\") then; Check.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedOtherInstanceForm",
            "Cfg.Set(''); if F.Run() then; Check.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedOtherGetLoadFields",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; Check.SetLoadFields(Qty); Check.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedOtherGetReadUncommitted",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; Check.ReadIsolation := IsolationLevel::ReadUncommitted; Check.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedOtherThenTryFails",
            "Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; if H.TryFail() then; H.Finish(Row, 'DB:RecordChanged');",
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
            "H.Seed(Row); Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Run\", Row) then; H.FinishDelete(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedGetThenDelete",
            "H.Seed(Row); Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Run\", Row) then; Check.Get('A'); H.FinishDelete(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedGetThenModifyTrue",
            "H.Seed(Row); Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Run\", Row) then; Check.Get('A'); H.FinishTrue(Row, 'DB:RecordChanged');",
            "refused",
        ),
        (
            "GuardedTwoGets",
            "H.Seed(Row); Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Run\", Row) then; Check.Get('A'); Check.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
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
