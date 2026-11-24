//! A `CurrentTransactionType` read while the type a test method before left is not known (al-runner issue 8969). Microsoft's
//! `Library - Manufacturing.RefreshProdOrder` commits, saves the type in a local, sets Update, runs a report, commits and
//! sets the saved type back: the value read is used only by that set, so the type after it is the one before the read,
//! still not known, and the test goes on. Every other use of the value, and a set back that does not take effect, stays
//! refused.

use al_builtins::assumptions::Assumption;
use al_testkit::outcomes_and_marks_in;
use al_testrun::Outcome;

const OBJECTS: &str = r#"
table 50410 "Saved Tx Row"
{
    DataClassification = CustomerContent;
    fields
    {
        field(1; "Entry No."; Integer) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

report 50411 "Saved Tx Update"
{
    ProcessingOnly = true;
    TransactionType = Update;

    dataset
    {
        dataitem(Loop; "Saved Tx Row") { }
    }
}

codeunit 50412 "Saved Tx Lib"
{
    var
        GlobalSaved: TransactionType;

    procedure SaveSetRestore()
    var
        Saved: TransactionType;
        UpdateReport: Report "Saved Tx Update";
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        UpdateReport.UseRequestPage := false;
        UpdateReport.RunModal();
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveCompared()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        if Saved = TransactionType::Update then;
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveToGlobal()
    begin
        Commit();
        GlobalSaved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        CurrentTransactionType(GlobalSaved);
    end;

    procedure SaveReassigned()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Saved := TransactionType::Update;
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SavePassedOn()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        SetBack(Saved);
    end;

    procedure SaveInIf()
    var
        Saved: TransactionType;
    begin
        Commit();
        if true then
            Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveRestoredTwice()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        CurrentTransactionType(Saved);
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveRestoreAfterWrite()
    var
        Saved: TransactionType;
        Row: Record "Saved Tx Row";
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Row."Entry No." := 7;
        Row.Insert();
        CurrentTransactionType(Saved);
    end;

    procedure SaveReadSetRestore()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        if CurrentTransactionType = TransactionType::Update then;
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveQualifiedReadSetRestore()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        if Database.CurrentTransactionType = TransactionType::Update then;
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveRestoreAfterRollback()
    var
        Saved: TransactionType;
        Row: Record "Saved Tx Row";
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Row."Entry No." := 8;
        Row.Insert();
        asserterror Error('x');
        CurrentTransactionType(Saved);
    end;

    local procedure SetBack(Saved: TransactionType)
    begin
        CurrentTransactionType(Saved);
    end;
}
"#;

/// The test before every shape: refused while it ran (an Update report after a read that follows the set is a moment no
/// cell measures), and what it did not run sets the type, so the type it leaves is not known.
const REFUSED_SETTER: &str = r#"
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "Saved Tx Row";
        UpdateReport: Report "Saved Tx Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
"#;

/// The test of a shape: `body`, then an Insert, which reads the type.
fn shape_test(body: &str) -> String {
    format!(
        r#"
    [Test]
    procedure B_Shape()
    var
        Lib: Codeunit "Saved Tx Lib";
        Row: Record "Saved Tx Row";
    begin
        {body}
        Row."Entry No." := 1;
        Row.Insert();
    end;
"#
    )
}

/// The outcome and marks of `B_Shape` running `body`, after the refused setter when `after_refused`.
fn run(body: &str, after_refused: bool) -> (Outcome, Vec<Assumption>) {
    let first = if after_refused { REFUSED_SETTER } else { "" };
    let src = format!(
        "{OBJECTS}\ncodeunit 50413 \"Saved Tx Tests\"\n{{\n    Subtype = Test;\n{first}{}\n}}\n",
        shape_test(body)
    );
    outcomes_and_marks_in(&src, "Saved Tx Tests")
        .into_iter()
        .find(|(n, ..)| n == "B_Shape")
        .map(|(_, o, m)| (o, m))
        .expect("B_Shape ran")
}

const NOT_KNOWN: &str = "after a test method of the codeunit that did not run to its end and could set it";

#[test]
fn a_type_saved_and_set_back_after_a_commit_runs_while_the_type_is_not_known() {
    let (o, marks) = run("Lib.SaveSetRestore();", true);
    assert_eq!(o, Outcome::Pass);
    assert!(
        marks.contains(&Assumption::TransactionTypeCarriesToNextTest),
        "{marks:?}"
    );
}

#[test]
fn a_type_saved_and_set_back_runs_as_before_when_the_type_is_known() {
    let (o, marks) = run("Lib.SaveSetRestore();", false);
    assert_eq!(o, Outcome::Pass);
    assert!(
        !marks.contains(&Assumption::TransactionTypeCarriesToNextTest),
        "{marks:?}"
    );
}

#[test]
fn the_type_is_not_known_again_after_the_set_back() {
    let (o, _) = run(
        "Lib.SaveSetRestore();\n        if CurrentTransactionType = TransactionType::Update then;",
        true,
    );
    match o {
        Outcome::NotRun(r) if r.contains(NOT_KNOWN) => {}
        other => panic!("expected the read after the set back to be refused, got {other:?}"),
    }
}

#[test]
fn every_other_use_of_the_saved_type_stays_refused() {
    let mut wrong = Vec::new();
    for call in [
        "Lib.SaveCompared();",
        "Lib.SaveToGlobal();",
        "Lib.SaveReassigned();",
        "Lib.SavePassedOn();",
        "Lib.SaveInIf();",
        "Lib.SaveRestoredTwice();",
        // A second read of the type in the procedure: the save's read is the only one.
        "Lib.SaveReadSetRestore();",
        "Lib.SaveQualifiedReadSetRestore();",
    ] {
        match run(call, true).0 {
            Outcome::NotRun(r) if r.contains(NOT_KNOWN) => {}
            other => wrong.push(format!("{call}: expected the read to be refused, got {other:?}")),
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn a_set_back_that_does_not_take_effect_is_refused() {
    match run("Lib.SaveRestoreAfterWrite();", true).0 {
        Outcome::NotRun(r)
            if r.contains("set back to the type read while the type a test method before left was not known") => {}
        other => panic!("expected the set back after a write to be refused, got {other:?}"),
    }
}

#[test]
fn a_set_back_after_a_rollback_since_the_read_is_refused() {
    match run("Lib.SaveRestoreAfterRollback();", true).0 {
        Outcome::NotRun(r)
            if r.contains("set back to the type read while the type a test method before left was not known") => {}
        other => panic!("expected the set back after a rollback to be refused, got {other:?}"),
    }
}
