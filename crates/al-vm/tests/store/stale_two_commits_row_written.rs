//! A stale buffer from the variable's own Insert, made stale by another variable's Modify and Commit in a later transaction,
//! is written whole (the other variable's change is lost) when the new transaction modified that same row through another
//! variable (al-runner issue 6522; record/lt-sb-stale-own-modify/Y27StaleOtherVarTests.al).

use al_builtins::assumptions::{clear_hits, take_hits};
use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 50380 "Stale Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { trigger OnValidate() begin "Validate Count" += 1; end; }
        field(3; "Remaining Quantity"; Decimal) { }
        field(4; "Validate Count"; Integer) { }
        field(5; "Modify Count"; Integer) { }
        field(6; "xRec Remaining Quantity"; Decimal) { }
        field(7; "xRec Description"; Text[50]) { }
    }
    keys { key(PK; "No.") { } }
    trigger OnModify()
    begin
        "Modify Count" += 1;
        "xRec Remaining Quantity" := xRec."Remaining Quantity";
        "xRec Description" := xRec.Description;
    end;
}
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

    procedure PostDescription(No: Code[20]; NewDescription: Text[50])
    var StaleRow: Record "Stale Row";
    begin
        StaleRow.Get(No);
        StaleRow.Description := NewDescription;
        StaleRow.Modify();
    end;

    procedure DeleteRow(No: Code[20])
    var StaleRow: Record "Stale Row";
    begin
        StaleRow.Get(No);
        StaleRow.Delete();
    end;

    procedure InsertAnotherRow(No: Code[20])
    var StaleRow: Record "Stale Row";
    begin
        StaleRow."No." := No;
        StaleRow.Description := 'Another row';
        StaleRow.Insert();
    end;
}
codeunit 50390 C
{
    var Poster: Codeunit Poster;

    local procedure InsertStale(var Row: Record "Stale Row"; Commits: Integer)
    var i: Integer;
    begin
        Row.DeleteAll();
        Row."No." := 'A';
        Row.Description := 'Original';
        Row."Remaining Quantity" := 100;
        Row.Insert(true);
        Commit();
        for i := 2 to Commits do
            Poster.PostAndCommit('A', 10);
    end;

    local procedure Fields(Row: Record "Stale Row"): Text
    begin
        exit(Row.Description + '|' + Format(Row."Remaining Quantity") + '|' + Format(Row."Validate Count") + '|' +
            Format(Row."Modify Count") + '|' + Format(Row."xRec Remaining Quantity") + '|' + Row."xRec Description");
    end;

    local procedure Stored(): Text
    var Row: Record "Stale Row";
    begin
        if not Row.Get('A') then
            exit('missing');
        exit(Fields(Row) + '|rows ' + Format(Row.Count()));
    end;

    local procedure Write(var Row: Record "Stale Row"): Text
    begin
        Row.Validate(Description, 'Changed');
        Row.Modify(true);
        exit(Fields(Row) + ' / ' + Stored());
    end;

    local procedure WriteError(var Row: Record "Stale Row"): Text
    begin
        Row.Validate(Description, 'Changed');
        asserterror Row.Modify(true);
        exit(GetLastErrorCode() + ' / ' + Fields(Row) + ' / ' + Stored());
    end;

    procedure ModifiesQuantity(Commits: Integer): Text
    var Row: Record "Stale Row";
    begin
        InsertStale(Row, Commits);
        Poster.Post('A', 5);
        exit(Write(Row));
    end;

    procedure ModifiesDescription(Commits: Integer): Text
    var Row: Record "Stale Row";
    begin
        InsertStale(Row, Commits);
        Poster.PostDescription('A', 'Posted');
        exit(Write(Row));
    end;

    procedure Deletes(Commits: Integer): Text
    var Row: Record "Stale Row";
    begin
        InsertStale(Row, Commits);
        Poster.DeleteRow('A');
        exit(WriteError(Row));
    end;

    procedure Nothing(Commits: Integer): Text
    var Row: Record "Stale Row";
    begin
        InsertStale(Row, Commits);
        exit(WriteError(Row));
    end;

    procedure UnchangedModify(): Text
    var Row: Record "Stale Row";
    begin
        InsertStale(Row, 2);
        Poster.Post('A', 0);
        exit(Write(Row));
    end;

    procedure ModifiesTwice(): Text
    var Row: Record "Stale Row";
    begin
        InsertStale(Row, 2);
        Poster.Post('A', 5);
        Poster.Post('A', 1);
        exit(Write(Row));
    end;

    procedure InsertsAnotherRow(): Text
    var Row: Record "Stale Row";
    begin
        InsertStale(Row, 2);
        Poster.InsertAnotherRow('B');
        exit(Write(Row));
    end;

    procedure ModifyFalse(): Text
    var Row: Record "Stale Row";
    begin
        InsertStale(Row, 2);
        Poster.Post('A', 5);
        Row.Description := 'Changed';
        Row.Modify(false);
        exit(Fields(Row) + ' / ' + Stored());
    end;

    procedure ModifiesAndInsertsAnotherRow(): Text
    var Row: Record "Stale Row";
    begin
        InsertStale(Row, 2);
        Poster.Post('A', 5);
        Poster.InsertAnotherRow('B');
        exit(Write(Row));
    end;

    procedure ModifiesUnderLock(): Text
    var Row: Record "Stale Row"; Locked: Record "Stale Row";
    begin
        InsertStale(Row, 2);
        Locked.LockTable();
        Poster.Post('A', 5);
        exit(Write(Row));
    end;

    procedure ModifiesThreeCommitsOneEmpty(): Text
    var Row: Record "Stale Row";
    begin
        InsertStale(Row, 1);
        Commit();
        Poster.PostAndCommit('A', 10);
        Poster.Post('A', 5);
        exit(Write(Row));
    end;

    procedure ModifiesFourCommits(): Text
    var Row: Record "Stale Row";
    begin
        InsertStale(Row, 4);
        Poster.Post('A', 5);
        exit(Write(Row));
    end;
}
"#;

const WHOLE: &str = "Changed|100|1|1|100|Changed / Changed|100|1|1|100|Changed|rows 1";

fn run(routine: &str, commits: Option<i64>) -> Result<al_builtins::value::Value, Signal> {
    match commits {
        Some(n) => {
            al_testkit::run_with(SRC, "C", routine, vec![al_builtins::value::Value::Integer(n as i32)]).map(|(v, _)| v)
        }
        None => al_testkit::run_in(SRC, "C", routine),
    }
}

/// Y27StaleOtherVarTests.al Y27_2C_* and Y27_3C_*: the whole stale copy is written, triggers once, xRec the buffer itself.
#[test]
fn the_stale_insert_copy_is_written_whole_after_another_variable_modified_the_row() {
    for n in [2, 3] {
        for p in ["ModifiesQuantity", "ModifiesDescription"] {
            clear_hits();
            assert_eq!(text(run(p, Some(n))), WHOLE, "{p} {n}");
            assert!(take_hits().is_empty(), "{p} {n}");
        }
    }
    for p in ["UnchangedModify", "ModifiesTwice"] {
        clear_hits();
        assert_eq!(text(run(p, None)), WHOLE, "{p}");
        assert!(take_hits().is_empty(), "{p}");
    }
    assert_eq!(
        text(run("ModifyFalse", None)),
        "Changed|100|0|0|0| / Changed|100|0|0|0||rows 1"
    );
}

/// Y27_1C_*: one Commit after the Insert the buffer is not stale.
#[test]
fn one_commit_after_the_insert_writes_the_buffer() {
    assert_eq!(text(run("ModifiesQuantity", Some(1))), WHOLE);
}

/// Y27_2C_OtherInsertsAnotherRow, decided by the al-runner issue 5954 branch.
#[test]
fn an_insert_of_another_row_writes_the_buffer() {
    assert_eq!(
        text(run("InsertsAnotherRow", None)),
        "Changed|100|1|1|100|Changed / Changed|100|1|1|100|Changed|rows 2"
    );
}

/// Y27_*_OtherDeletes and Y27_2C_Nothing, Y27_3C_Nothing (:269-299): OnValidate and OnModify run on the buffer, with xRec
/// the buffer, before the error.
#[test]
fn a_deleted_row_or_no_contact_raises() {
    for n in [1, 2, 3] {
        let committed = 110 - 10 * n;
        assert_eq!(
            text(run("Deletes", Some(n))),
            format!("DB:RecordNotFound / Changed|100|1|1|100|Changed / Original|{committed}|0|0|0||rows 1"),
            "{n}"
        );
    }
    for n in [2, 3] {
        let committed = 110 - 10 * n;
        assert_eq!(
            text(run("Nothing", Some(n))),
            format!("DB:RecordChanged / Changed|100|1|1|100|Changed / Original|{committed}|0|0|0||rows 1"),
            "{n}"
        );
    }
}

/// Shapes no test pins stay refused.
#[test]
fn other_writes_a_lock_or_more_commits_stay_refused() {
    for p in [
        "ModifiesAndInsertsAnotherRow",
        "ModifiesUnderLock",
        "ModifiesThreeCommitsOneEmpty",
        "ModifiesFourCommits",
    ] {
        let r = run(p, None);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{p}: {r:?}");
    }
}
