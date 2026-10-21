//! SessionInformation.SqlStatementsExecuted and SqlRowsRead (AL Runner issue 5580), pinned by
//! session/lt-bl-sql-counters/SqlCounters.al: not negative, BigInteger, and how they change over one operation. A read after an
//! operation whose change no corpus test pins is refused.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

fn run(src: &str, routine: &str) -> Result<Value, Signal> {
    al_testkit::run_in(src, "C", routine)
}

const SRC: &str = r#"
table 50420 "Sql Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
}
codeunit 50421 C
{
    var
        StmtBefore: BigInteger;
        RowsBefore: BigInteger;
        R: Text;

    local procedure Seed()
    var
        Row: Record "Sql Row";
        I: Integer;
    begin
        Row.DeleteAll();
        for I := 1 to 3 do begin
            Row.Id := I;
            Row.Name := 'row ' + Format(I);
            Row.Insert();
        end;
        Commit();
    end;

    local procedure Mark()
    begin
        StmtBefore := SessionInformation.SqlStatementsExecuted();
        RowsBefore := SessionInformation.SqlRowsRead();
    end;

    local procedure Sign(Delta: BigInteger): Text
    begin
        if Delta > 0 then exit('increased');
        if Delta < 0 then exit('decreased');
        exit('same');
    end;

    local procedure Note(Name: Text)
    var
        Stmt: BigInteger;
        Rows: BigInteger;
    begin
        Stmt := SessionInformation.SqlStatementsExecuted();
        Rows := SessionInformation.SqlRowsRead();
        R += Name + ': statements ' + Sign(Stmt - StmtBefore) + ', rows ' + Sign(Rows - RowsBefore) + '\';
    end;

    procedure NotNegative(): Text
    var
        V: Variant;
    begin
        R := Format(SessionInformation.SqlStatementsExecuted() >= 0) + Format(SessionInformation.SqlRowsRead() >= 0);
        V := SessionInformation.SqlStatementsExecuted();
        R += Format(V.IsBigInteger()) + Format(V.IsInteger());
        V := SessionInformation.SqlRowsRead();
        exit(R + Format(V.IsBigInteger()) + Format(V.IsInteger()));
    end;

    procedure NothingBetween(): Text
    var
        X: Integer;
    begin
        Mark();
        Note('two reads');
        Mark();
        X := 1 + 2;
        Note('arithmetic only');
        exit(R);
    end;

    procedure TableOperations(): Text
    var
        Row: Record "Sql Row";
        N: Integer;
    begin
        Seed();
        Mark();
        Row.Get(2);
        Note('get');
        Mark();
        Row.FindSet();
        repeat
        until Row.Next() = 0;
        Note('findset loop');
        Mark();
        N := Row.Count();
        Note('count');
        Mark();
        Row.Id := 10;
        Row.Name := 'ten';
        Row.Insert();
        Note('insert');
        Mark();
        Row.Name := 'TEN';
        Row.Modify();
        Note('modify');
        Mark();
        Row.Delete();
        Note('delete');
        exit(R);
    end;

    procedure TemporaryOperations(): Text
    var
        Row: Record "Sql Row" temporary;
        N: Integer;
    begin
        Row.Id := 1; Row.Name := 'one'; Row.Insert();
        Row.Id := 2; Row.Name := 'two'; Row.Insert();
        Mark(); Row.Get(2); Note('get');
        Mark(); Row.FindSet(); repeat until Row.Next() = 0; Note('findset loop');
        Mark(); N := Row.Count(); Note('count');
        Mark(); Row.Id := 10; Row.Insert(); Note('insert');
        Mark(); Row.Name := 'TEN'; Row.Modify(); Note('modify');
        Mark(); Row.Delete(); Note('delete');
        exit(R);
    end;

    procedure UnpinnedBetween(): Text
    var
        Row: Record "Sql Row";
    begin
        Mark();
        Row.DeleteAll();
        Note('deleteall');
        exit(R);
    end;
}
"#;

/// SqlCounters.al:74 (Counters_NotNegative) and :86 (Counters_Type).
#[test]
fn counters_are_not_negative_big_integers() {
    assert_eq!(text(run(SRC, "NotNegative")), "YesYesYesNoYesNo");
}

/// SqlCounters.al:105 (Counters_NothingBetween).
#[test]
fn counters_stay_with_nothing_between() {
    assert_eq!(
        text(run(SRC, "NothingBetween")),
        "two reads: statements same, rows same\\arithmetic only: statements same, rows same\\"
    );
}

/// SqlCounters.al:121 (Counters_TableOperations).
#[test]
fn counters_change_over_table_operations() {
    assert_eq!(
        text(run(SRC, "TableOperations")),
        [
            "get: statements increased, rows increased",
            "findset loop: statements increased, rows increased",
            "count: statements same, rows same",
            "insert: statements same, rows same",
            "modify: statements increased, rows increased",
            "delete: statements increased, rows same",
            "",
        ]
        .join("\\")
    );
}

/// SqlCounters.al:161 (Counters_TemporaryOperations).
#[test]
fn counters_stay_over_temporary_operations() {
    assert_eq!(
        text(run(SRC, "TemporaryOperations")),
        [
            "get: statements same, rows same",
            "findset loop: statements same, rows same",
            "count: statements same, rows same",
            "insert: statements same, rows same",
            "modify: statements same, rows same",
            "delete: statements same, rows same",
            "",
        ]
        .join("\\")
    );
}

/// How a DeleteAll changes the counters is not pinned: the read after it is refused.
#[test]
fn a_read_after_an_unpinned_operation_is_refused() {
    match run(SRC, "UnpinnedBetween") {
        Err(Signal::Unsupported(w)) => assert!(w.contains("SqlStatementsExecuted"), "{w}"),
        other => panic!("{other:?}"),
    }
}
