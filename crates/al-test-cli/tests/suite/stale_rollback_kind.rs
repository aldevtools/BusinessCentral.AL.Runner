//! A buffer made stale by its own write, then a rollback or a locking ReadIsolation read in the open transaction, then the
//! buffer's Modify. Only the measured shapes run; every other shape is refused as before corpus PR 1323 (review of runner PR 7800).
//!
//! Measured: after `asserterror Error('literal')`, nothing, or a Get and Modify of the row (StaleRuleMatrix.al:206 Rollback);
//! after a failed guarded `Codeunit.Run` that undid no write, nothing or a Get of the row alone
//! (record/crud/CodeunitRunTableNoWrites.al:205, :267); a locking read (Get, FindSet of its table, FindFirst of another table)
//! beside nothing or beside writes of other rows of its table (CodeunitRunTableNoWrites.al:446-545, :629-741), and the
//! four-Commit case of record/WbStaleUpdLockTests.al:16-25.
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
        src.push_str(&format!(
            "codeunit {} \"{name}\"\n{{\n    Subtype = Test;\n    TestPermissions = Disabled;\n    [Test] procedure {name}()\n    var Row: Record \"P Row\"; Check: Record \"P Row\"; R: Record \"P Row\"; S: Record \"P Side\"; H: Codeunit \"P Help\"; Cfg: Codeunit \"P Cfg\";\n    begin\n        H.MakeStale(Row);\n        {body}\n    end;\n}}\n",
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
        // Both kinds in one transaction are not measured.
        (
            "AssertThenGuarded",
            "asserterror Error('x'); Cfg.Set(''); if Codeunit.Run(Codeunit::\"P Fail\") then; Check.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "refused",
        ),
    ]);
}

#[test]
fn only_the_measured_locking_reads_run() {
    check(&[
        // Measured: a locking Get or FindSet of its table with nothing else (CodeunitRunTableNoWrites.al:446-545, :629-741).
        (
            "UpdLockGetOtherRow",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.Get('B'); H.Finish(Row, 'DB:RecordChanged');",
            "pass",
        ),
        (
            "UpdLockGetOwnRow",
            "R.ReadIsolation := IsolationLevel::UpdLock; R.Get('A'); H.Finish(Row, 'DB:RecordChanged');",
            "pass",
        ),
        (
            "RepeatableReadFindSet",
            "R.ReadIsolation := IsolationLevel::RepeatableRead; R.SetRange(\"No.\", 'B'); R.FindSet(); H.Finish(Row, 'DB:RecordChanged');",
            "pass",
        ),
        (
            "UpdLockFindFirstOtherTable",
            "S.ReadIsolation := IsolationLevel::UpdLock; if S.FindFirst() then; H.Finish(Row, 'DB:RecordChanged');",
            "pass",
        ),
        // Not measured: another read form, a LockTable beside it, a write of another table, row contact.
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
    ]);
}
