//! The size of the change of `SessionInformation.SqlStatementsExecuted()` and `SqlRowsRead()` around a statement budget
//! test (al-runner issue 5861): a procedure in another codeunit copies a temporary record into a database table of a
//! two-field key with FindSet, Next and Insert, then SetRange and Count, with or without a Commit before the first read
//! (session/lt-bl-sql-counters/SkSqlBudget.al). Any other shape, N or buffer state refuses.

use al_builtins::native::Signal;
use al_testkit::{Built, run, text};

/// The objects of SkSqlBudget.al:14-66 and its `Measure` (:104-133).
fn src(extra: &str) -> String {
    format!(
        "table 50100 RuleLine {{ fields {{ field(1; \"Group Code\"; Code[20]) {{ }} field(2; \"Line No.\"; Integer) {{ }}
            field(3; Value; Text[50]) {{ }} field(4; Weight; Integer) {{ }} }} keys {{ key(PK; \"Group Code\", \"Line No.\") {{ }} }} }}
table 50103 OtherLine {{ fields {{ field(1; \"Group Code\"; Code[20]) {{ }} field(2; \"Line No.\"; Integer) {{ }}
            field(3; Value; Text[30]) {{ }} }} keys {{ key(PK; \"Group Code\", \"Line No.\") {{ }} }} }}
codeunit 50101 Builder {{
    procedure CopyLines(GroupCode: Code[20]; var TempRuleLine: Record RuleLine temporary): Integer
    var RuleLine: Record RuleLine;
    begin
        if TempRuleLine.FindSet() then
            repeat RuleLine := TempRuleLine; RuleLine.\"Group Code\" := GroupCode; RuleLine.Insert(); until TempRuleLine.Next() = 0;
        RuleLine.SetRange(\"Group Code\", GroupCode);
        exit(RuleLine.Count());
    end;
    procedure CopyLinesOnly(GroupCode: Code[20]; var TempRuleLine: Record RuleLine temporary)
    var RuleLine: Record RuleLine;
    begin
        if TempRuleLine.FindSet() then
            repeat RuleLine := TempRuleLine; RuleLine.\"Group Code\" := GroupCode; RuleLine.Insert(); until TempRuleLine.Next() = 0;
    end;
    procedure CountGroup(GroupCode: Code[20]): Integer
    var RuleLine: Record RuleLine;
    begin RuleLine.SetRange(\"Group Code\", GroupCode); exit(RuleLine.Count()); end;
}}
codeunit 50102 T {{
    var Seq: Integer;
    local procedure FillBuffer(var TempRuleLine: Record RuleLine temporary; N: Integer)
    var i: Integer;
    begin
        TempRuleLine.Reset(); TempRuleLine.DeleteAll();
        for i := 1 to N do begin
            TempRuleLine.Init(); TempRuleLine.\"Group Code\" := 'TEMP'; TempRuleLine.\"Line No.\" := i;
            TempRuleLine.Value := 'Value'; TempRuleLine.Weight := i; TempRuleLine.Insert();
        end;
    end;
    procedure Measure(Kind: Integer; N: Integer; WithCommit: Boolean): Text
    var TempRuleLine: Record RuleLine temporary; B: Codeunit Builder; StmtsBefore: BigInteger; RowsBefore: BigInteger;
        StmtsDelta: BigInteger; RowsDelta: BigInteger; Group: Code[20]; Copied: Integer;
    begin
        FillBuffer(TempRuleLine, N);
        Seq += 1; Group := 'SK' + Format(Seq);
        if WithCommit then Commit();
        StmtsBefore := SessionInformation.SqlStatementsExecuted();
        RowsBefore := SessionInformation.SqlRowsRead();
        case Kind of
            1: Copied := B.CopyLines(Group, TempRuleLine);
            2: B.CopyLinesOnly(Group, TempRuleLine);
            3: begin B.CopyLinesOnly(Group, TempRuleLine); Copied := B.CountGroup(Group); end;
        end;
        StmtsDelta := SessionInformation.SqlStatementsExecuted() - StmtsBefore;
        RowsDelta := SessionInformation.SqlRowsRead() - RowsBefore;
        if (Kind <> 2) and (Copied <> N) then Error('Copied %1 rows, expected %2.', Copied, N);
        exit(Format(StmtsDelta) + '/' + Format(RowsDelta));
    end;
    procedure Repeat3(Kind: Integer; N: Integer; WithCommit: Boolean): Text
    begin exit(Measure(Kind, N, WithCommit) + ' ' + Measure(Kind, N, WithCommit) + ' ' + Measure(Kind, N, WithCommit)); end;
    procedure Series(Kind: Integer; WithCommit: Boolean): Text
    begin
        exit(Repeat3(Kind, 1, WithCommit) + ',' + Repeat3(Kind, 2, WithCommit) + ',' + Repeat3(Kind, 5, WithCommit) + ','
            + Repeat3(Kind, 10, WithCommit) + ',' + Repeat3(Kind, 20, WithCommit));
    end;
    {extra}
}}"
    )
}

fn refused(r: Result<al_builtins::value::Value, Signal>) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains("SQL counter"), "{why}"),
        other => panic!("expected a SQL counter refusal, got {other:?}"),
    }
}

fn series(kind: u32, commit: bool) -> String {
    text(run(
        &src(&format!(
            "procedure F(): Text begin exit(Series({kind}, {commit})); end;"
        )),
        "F",
    ))
}

/// SkSqlBudget.al:215-256 and :284-298: copy and Count, in one call or two, with and without a Commit first.
#[test]
fn copy_and_count_give_the_measured_sizes() {
    let want = "2/1 2/1 2/1,3/1 3/1 3/1,2/1 2/1 2/1,3/1 3/1 3/1,5/1 5/1 5/1";
    assert_eq!(series(1, false), want);
    assert_eq!(series(1, true), want);
    assert_eq!(series(3, false), want);
}

/// SkSqlBudget.al:271-280: the copy alone, right after a Commit (inserts are buffered).
#[test]
fn copy_only_after_a_commit_gives_the_measured_sizes() {
    assert_eq!(
        series(2, true),
        "0/0 0/0 0/0,0/0 0/0 0/0,1/0 1/0 1/0,2/0 2/0 2/0,4/0 4/0 4/0"
    );
}

/// The reporter's tests (SkSqlBudget.al:140-208): no Commit, the difference is a plain number, compared and formatted.
#[test]
fn the_reporters_budget_test_passes() {
    let s = src("procedure F(): Text
        var TempRuleLine: Record RuleLine temporary; B: Codeunit Builder; Before: BigInteger; After: BigInteger; Executed: BigInteger;
        begin
            FillBuffer(TempRuleLine, 10);
            Before := SessionInformation.SqlStatementsExecuted();
            if B.CopyLines('BUDGET1', TempRuleLine) <> 10 then Error('copied');
            After := SessionInformation.SqlStatementsExecuted();
            Executed := After - Before;
            exit(Format(Executed <= 1000) + ' ' + StrSubstNo('SQL statements: %1, expected <= 1000', Executed));
        end;");
    assert_eq!(text(run(&s, "F")), "Yes SQL statements: 3, expected <= 1000");
}

/// Inserts still buffered from an earlier copy change the next measurement (SkSqlBudget.al:9-10, :273): the copy after a
/// copy without Count and without Commit counts them, and a copy after database inserts made before the session's first
/// read, whose buffer is not known, is not sized.
#[test]
fn a_copy_with_inserts_still_buffered_is_refused() {
    let mut b = Built::from_source(&src("procedure CopyOnly(): Text begin exit(Measure(2, 1, false)); end;
"));
    assert_eq!(text(b.run("CopyOnly")), "0/0");
    // Once the session has inserted into the table, a copy after a copy without Count and without Commit counts the row
    // the first left buffered (SkSqlBudget.al:264-266, :272-273 "copy N=1 0/0 0/0 0/0", "copy N=2 1/0 0/0 0/0").
    assert_eq!(
        text(
            Built::from_source(&src(
                "procedure F(): Text begin exit(Measure(2, 1, false) + Measure(2, 1, false)); end;",
            ))
            .run("F")
        ),
        "0/00/0"
    );
    // A copy of two rows and a Count with one row left buffered: the Count sends three rows, one statement each
    // (SkSqlBudget.al:449 "gt count 3 4/1").
    assert_eq!(
        text(
            Built::from_source(&src(
                "procedure F(): Text begin exit(Measure(2, 1, false) + Measure(1, 2, false)); end;",
            ))
            .run("F")
        ),
        "0/04/1"
    );
    refused(Built::from_source(&src(
        "procedure InsertFirst(): Text var R: Record RuleLine; begin R.\"Group Code\" := 'X'; R.\"Line No.\" := 1; R.Insert(); exit(Measure(1, 2, false)); end;",
    ))
    .run("InsertFirst"));
}

/// An N the corpus does not measure, another table layout, inserts not made by the copy loop.
#[test]
fn an_unmeasured_copy_is_refused() {
    refused(run(
        &src("procedure F(): Text begin exit(Measure(1, 3, true)); end;"),
        "F",
    ));
    refused(run(
        &src("procedure F(): Text begin exit(Measure(2, 4, true)); end;"),
        "F",
    ));
    let other = src("procedure F(): Text
        var Temp: Record OtherLine temporary; L: Record OtherLine; Before: BigInteger; Cnt: Integer;
        begin
            Temp.\"Line No.\" := 1; Temp.Insert();
            Before := SessionInformation.SqlStatementsExecuted();
            if Temp.FindSet() then repeat L := Temp; L.\"Group Code\" := 'G'; L.Insert(); until Temp.Next() = 0;
            L.SetRange(\"Group Code\", 'G'); Cnt := L.Count();
            exit(Format(SessionInformation.SqlStatementsExecuted() - Before));
        end;");
    refused(run(&other, "F"));
    let direct = src("procedure F(): Text
        var L: Record RuleLine; Before: BigInteger; Cnt: Integer;
        begin
            Before := SessionInformation.SqlStatementsExecuted();
            L.\"Group Code\" := 'G'; L.\"Line No.\" := 1; L.Insert();
            L.SetRange(\"Group Code\", 'G'); Cnt := L.Count();
            exit(Format(SessionInformation.SqlStatementsExecuted() - Before));
        end;");
    refused(run(&direct, "F"));
}
