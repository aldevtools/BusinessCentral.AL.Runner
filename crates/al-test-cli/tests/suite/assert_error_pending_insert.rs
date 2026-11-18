//! An `asserterror` whose body ends without an error while an Insert error is deferred (al-runner issue 8267): a duplicate
//! key, or a Date its SQL column cannot hold (records/sql_range.rs). A body left by exit, break or continue is refused by the
//! lowering (assert_error_leaves.rs).
//! The statement-form Insert of an existing key into a table without a Blob or AutoIncrement field returns without an error,
//! and the next access to the table or a Commit raises it (record/TestRecordRuntimeTextsAndViews.al:189-210;
//! record/duplicate-insert/Y7407DuplicateInsertBuffer.al:98-123, the statement after the Insert inside the asserterror is
//! reached). Whether the end of an asserterror body that raised nothing sends the pending duplicate is not measured: refused.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

/// The reporter's repro, unchanged.
const REPRO: &str = r#"
table 51900 "Dup Row"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Name; Text[50]) { }
    }
    keys { key(PK; Code) { Clustered = true; } }
}

codeunit 51950 "Dup Bare"
{
    Subtype = Test;

    [Test]
    procedure BareInsertTwiceRaises()          // FAIL: An error was expected inside an ASSERTERROR statement.
    var
        R: Record "Dup Row";
    begin
        R.Code := 'A';
        R.Insert();
        asserterror R.Insert();
    end;

    [Test]
    procedure InsertSameKeyFromSecondVariable() // FAIL: the same
    var
        R: Record "Dup Row";
        R2: Record "Dup Row";
    begin
        R.Code := 'A';
        R.Insert();
        R2.Code := 'A';
        asserterror R2.Insert();
    end;

    [Test]
    procedure InsertTwiceReturnsFalse()         // passes: the return-value form answers false, as BC does
    var
        R: Record "Dup Row";
    begin
        R.Code := 'A';
        R.Insert();
        if R.Insert() then
            Error('the second Insert of key A returned true');
    end;
}
"#;

/// Neighbors: the measured shape (the table is read inside the asserterror), a pending duplicate from before an asserterror
/// whose body raises nothing, and an asserterror whose body raises.
const NEIGHBORS: &str = r#"
table 51901 "Dup Row N"
{
    fields { field(1; Code; Code[20]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
table 51902 "Dup Blob Row"
{
    fields { field(1; Code; Code[20]) { } field(2; Data; Blob) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
codeunit 51951 "Dup Neighbors"
{
    Subtype = Test;

    [Test]
    procedure ReadInsideTheAssertError()
    var
        R: Record "Dup Row N";
        Reached: Boolean;
    begin
        R.Code := 'A';
        R.Insert();
        asserterror begin
            R.Insert();
            Reached := true;
            R.Count();
        end;
        if not Reached then
            Error('the duplicate Insert raised at once');
        if GetLastErrorCode() <> 'DB:RecordExists' then
            Error('code: %1', GetLastErrorCode());
    end;

    [Test]
    procedure PendingBeforeAnAssertErrorThatRaisesNothing()
    var
        R: Record "Dup Row N";
        I: Integer;
    begin
        R.Code := 'A';
        R.Insert();
        R.Insert();
        asserterror I := 1;
    end;

    [Test]
    procedure PendingBeforeAnAssertErrorThatRaises()
    var
        R: Record "Dup Row N";
    begin
        R.Code := 'A';
        R.Insert();
        R.Insert();
        asserterror Error('x');
        R.Count();
    end;

    [Test]
    procedure BlobTableRaisesAtTheInsert()
    var
        R: Record "Dup Blob Row";
    begin
        R.Code := 'A';
        R.Insert();
        asserterror R.Insert();
        if GetLastErrorCode() <> 'DB:RecordExists' then
            Error('code: %1', GetLastErrorCode());
    end;

    [Test]
    procedure NothingPendingAssertErrorRaisesNothing()
    var
        I: Integer;
    begin
        asserterror I := 1;
    end;
}
"#;

fn of(out: &[(String, Outcome)], n: &str) -> Outcome {
    out.iter()
        .find(|(t, _)| t == n)
        .map(|(_, o)| o.clone())
        .unwrap_or_else(|| panic!("no outcome for {n}: {out:?}"))
}

/// The reason of a refusal; panics on any other outcome.
fn reason(o: Outcome) -> String {
    match o {
        Outcome::NotRun(r) => r,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn refused(o: Outcome) {
    let r = reason(o);
    assert!(r.contains("asserterror"), "{r}");
}

#[test]
fn issue_8267_an_assert_error_that_ends_with_its_duplicate_insert_pending_is_refused() {
    let out = outcomes_in(REPRO, "Dup Bare");
    refused(of(&out, "BareInsertTwiceRaises"));
    refused(of(&out, "InsertSameKeyFromSecondVariable"));
    assert_eq!(of(&out, "InsertTwiceReturnsFalse"), Outcome::Pass);
}

#[test]
fn a_duplicate_pending_from_before_an_assert_error_that_raises_nothing_is_refused() {
    let out = outcomes_in(NEIGHBORS, "Dup Neighbors");
    refused(of(&out, "PendingBeforeAnAssertErrorThatRaisesNothing"));
}

/// A deferred Insert error of the other kind: a Date the SQL column cannot hold (records/sql_range.rs `insert_sql_range`).
const DATE_KIND: &str = r#"
table 51903 "Dup Dated Row"
{
    fields { field(1; Code; Code[20]) { } field(2; Dt; Date) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
codeunit 51952 "Dup Dated"
{
    Subtype = Test;

    [Test]
    procedure DateTheColumnCannotHoldPending()
    var
        D: Record "Dup Dated Row";
    begin
        D.Code := 'A';
        D.Dt := 17000101D;
        asserterror D.Insert();
    end;
}
"#;

#[test]
fn the_refusal_names_both_kinds_of_deferred_insert_error() {
    let out = outcomes_in(DATE_KIND, "Dup Dated");
    let r = reason(of(&out, "DateTheColumnCannotHoldPending"));
    assert!(r.contains("Dup Dated Row"), "{r}");
    assert!(
        r.contains("a duplicate key, or a value its SQL column cannot hold"),
        "{r}"
    );
}

/// Main's result, kept for the duplicate kind (probe queued, docs/probes-needed.md): the error of the body is caught and the
/// pending Insert error is dropped. Pinned for the other kind only: with a deferred SQL-range Insert error, a read of another
/// table raises nothing and the following Error() is what the asserterror catches (record/filters/SqlDateFloor1753.al:1920-1935).
#[test]
fn a_body_that_raises_another_error_with_the_duplicate_pending_keeps_main_s_result() {
    let out = outcomes_in(NEIGHBORS, "Dup Neighbors");
    assert_eq!(of(&out, "PendingBeforeAnAssertErrorThatRaises"), Outcome::Pass);
}

#[test]
fn the_measured_neighbors_keep_their_results() {
    let out = outcomes_in(NEIGHBORS, "Dup Neighbors");
    assert_eq!(of(&out, "ReadInsideTheAssertError"), Outcome::Pass);
    assert_eq!(of(&out, "BlobTableRaisesAtTheInsert"), Outcome::Pass);
    assert_eq!(
        of(&out, "NothingPendingAssertErrorRaisesNothing"),
        Outcome::Fail("An error was expected inside an ASSERTERROR statement.".into())
    );
}
