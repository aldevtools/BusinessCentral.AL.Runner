//! Exact sizes of the change of `SessionInformation.SqlStatementsExecuted()` and `SqlRowsRead()` (al-runner issue 5761).
//! The corpus pins them for nine shapes of record methods on a two-field table, each measured right after a Commit that
//! left the table empty or with N rows, for N = 1, 2, 5, 10, 20 (session/lt-bl-sql-counters/SqlCounters.al:204-395).
//! Any other shape, N, table or precondition keeps only the sign of the change, or refuses.

use al_builtins::native::Signal;
use al_testkit::{run, run_in, run_with, text};

const TABLE: &str = "table 50100 Row { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }";

/// The corpus's RoReset and RoMeasure (SqlCounters.al:206-276), on table `Row`.
fn src(extra: &str) -> String {
    format!(
        "{TABLE}
table 50102 Wide {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[30]) {{ }} field(3; Extra; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 50101 C {{
    var StmtBefore: BigInteger; RowsBefore: BigInteger;
    local procedure Mark() begin StmtBefore := SessionInformation.SqlStatementsExecuted(); RowsBefore := SessionInformation.SqlRowsRead(); end;
    local procedure RoReset(Rows: Integer)
    var Row: Record Row; I: Integer;
    begin
        Row.DeleteAll();
        for I := 1 to Rows do begin Row.Id := I; Row.Name := 'row ' + Format(I); Row.Insert(); end;
        Commit();
    end;
    local procedure Measured(): Text
    var Stmt: BigInteger; Rows: BigInteger;
    begin
        Stmt := SessionInformation.SqlStatementsExecuted();
        Rows := SessionInformation.SqlRowsRead();
        exit(Format(Stmt - StmtBefore) + '/' + Format(Rows - RowsBefore));
    end;
    procedure RoMeasure(Kind: Integer; N: Integer): Text
    var Row: Record Row; I: Integer; Cnt: Integer;
    begin
        if Kind in [1, 2] then RoReset(0) else RoReset(N);
        Mark();
        case Kind of
            1, 2:
                begin
                    for I := 1 to N do begin Row.Init(); Row.Id := 100 + I; Row.Name := 'Row'; Row.Insert(); end;
                    if Kind = 2 then Cnt := Row.Count();
                end;
            3: for I := 1 to N do Row.Get(I);
            4: for I := 1 to N do Row.Get(1);
            5: if Row.FindSet() then repeat until Row.Next() = 0;
            6: for I := 1 to N do begin Row.Get(I); Row.Name := 'changed'; Row.Modify(); end;
            7: for I := 1 to N do begin Row.Get(I); Row.Delete(); end;
            8: Row.DeleteAll();
            9: Cnt := Row.Count();
        end;
        exit(Measured());
    end;
    local procedure Twice(Kind: Integer; N: Integer): Text begin exit(RoMeasure(Kind, N) + ',' + RoMeasure(Kind, N)); end;
    procedure Series(Kind: Integer): Text
    begin
        exit(Twice(Kind, 1) + ' ' + Twice(Kind, 2) + ' ' + Twice(Kind, 5) + ' ' + Twice(Kind, 10) + ' ' + Twice(Kind, 20));
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

fn series(kind: u32) -> String {
    text(run(
        &src(&format!("procedure F(): Text begin exit(Series({kind})); end;")),
        "F",
    ))
}

fn twice(values: [&str; 5]) -> String {
    values.map(|v| format!("{v},{v}")).join(" ")
}

/// SqlCounters.al:323-334: inserts are buffered; a Count after them flushes.
#[test]
fn inserts_and_inserts_with_count_give_the_measured_sizes() {
    assert_eq!(series(1), twice(["0/0", "0/0", "1/0", "2/0", "4/0"]));
    assert_eq!(series(2), twice(["2/1", "3/1", "2/1", "3/1", "5/1"]));
}

/// SqlCounters.al:345-359: Get of distinct rows, Get of the same row, a FindSet loop.
#[test]
fn reads_give_the_measured_sizes() {
    assert_eq!(series(3), twice(["1/1", "2/2", "5/5", "10/10", "20/20"]));
    assert_eq!(series(4), twice(["1/1"; 5]));
    assert_eq!(series(5), twice(["1/2", "1/3", "1/6", "1/11", "1/21"]));
}

/// SqlCounters.al:371-390: Get and Modify, Get and Delete, DeleteAll, Count.
#[test]
fn writes_and_count_give_the_measured_sizes() {
    assert_eq!(series(6), twice(["2/2", "4/4", "10/10", "20/20", "40/40"]));
    assert_eq!(series(7), twice(["2/1", "4/2", "10/5", "20/10", "40/20"]));
    assert_eq!(series(8), twice(["1/0"; 5]));
    assert_eq!(series(9), twice(["1/1"; 5]));
}

/// The reporter's shape (SqlCounters.al:282-305): 10 inserts and a Count right after a Commit of the emptied table.
#[test]
fn the_statement_budget_repro_passes() {
    let s = src("procedure F(): Text var Row: Record Row; Before: BigInteger; After: BigInteger; Executed: BigInteger; I: Integer;
        begin
            Row.DeleteAll();
            Commit();
            Before := SessionInformation.SqlStatementsExecuted();
            for I := 1 to 10 do begin Row.Init(); Row.Id := I; Row.Name := 'Row'; Row.Insert(); end;
            if Row.Count() <> 10 then Error('Expected 10 rows.');
            After := SessionInformation.SqlStatementsExecuted();
            Executed := After - Before;
            exit(Format((Executed > 0) and (Executed <= 100)) + ' ' + Format(Executed));
        end;");
    assert_eq!(text(run(&s, "F")), "Yes 3");
}

/// An N the corpus does not measure (3, 0, 21) keeps the size unknown.
#[test]
fn an_unmeasured_n_is_refused() {
    for (kind, n) in [(1, 3), (2, 3), (8, 3)] {
        refused(run(
            &src(&format!("procedure F(): Text begin exit(RoMeasure({kind}, {n})); end;")),
            "F",
        ));
    }
}

/// The measurement must start right after the Commit: work between the Commit and the first read, or no Commit at all.
#[test]
fn a_measurement_not_right_after_a_commit_is_refused() {
    refused(run(
        &src("procedure F(): Text var Row: Record Row; I: Integer;
            begin RoReset(0); Row.Id := 1; Row.Insert(); Mark();
            for I := 2 to 3 do begin Row.Init(); Row.Id := I; Row.Insert(); end; exit(Measured()); end;"),
        "F",
    ));
    refused(run(
        &src("procedure F(): Text var Row: Record Row; I: Integer;
            begin Row.DeleteAll(); Mark(); for I := 1 to 2 do begin Row.Init(); Row.Id := I; Row.Insert(); end; exit(Measured()); end;"),
        "F",
    ));
}

/// The table must hold what the measurement started from: N rows for the reads and writes, none for the inserts.
#[test]
fn a_table_in_another_state_is_refused() {
    // Inserts into a table that already holds 5 rows.
    refused(run(
        &src("procedure F(): Text var Row: Record Row; I: Integer;
            begin RoReset(5); Mark(); for I := 1 to 2 do begin Row.Init(); Row.Id := 100 + I; Row.Insert(); end; exit(Measured()); end;"),
        "F",
    ));
    // Three Gets in a table of 5 rows, two of one row (the second may come from BC's cache).
    refused(run(
        &src(
            "procedure F(): Text var Row: Record Row; begin RoReset(5); Mark(); Row.Get(1); Row.Get(1); Row.Get(2); exit(Measured()); end;",
        ),
        "F",
    ));
}

/// Other shapes: a filter, inserts without Init, a FindSet loop stopped early, a mix of kinds, another table layout.
#[test]
fn an_unmeasured_shape_is_refused() {
    for body in [
        "RoReset(0); Mark(); Row.Id := 1; Row.Insert(); Row.Id := 2; Row.Insert();",
        "RoReset(2); Mark(); Row.FindSet(); Row.Next();",
        "RoReset(2); Mark(); Row.Get(1); Cnt := Row.Count();",
        "RoReset(1); Mark(); Row.Get(1); Row.Name := 'x'; Row.Modify(true);",
        "Wide.DeleteAll(); Commit(); Mark(); Wide.Init(); Wide.Id := 1; Wide.Insert(); Wide.Init(); Wide.Id := 2; Wide.Insert(); Cnt := Wide.Count();",
    ] {
        let s = src(&format!(
            "procedure F(): Text var Row: Record Row; Wide: Record Wide; Cnt: Integer; begin {body} exit(Measured()); end;"
        ));
        refused(run(&s, "F"));
    }
}
/// On this table, and for an N or filter the corpus did not measure, each record method costs what writer TX measured it to
/// cost alone on its table (session/lt-bl-sql-counters/SkSqlBudget.al:709-860): Get 1/1, a FindSet loop 1/(N+1), Get and Modify
/// 2/2 a row, Count 1/1. The pass is flagged: it rests on `sql-read-cost-unmeasured-shape`.
#[test]
fn an_unmeasured_shape_or_n_costs_as_measured_and_is_flagged() {
    use al_builtins::assumptions::{Assumption, take_hits};
    for (kind, n, want) in [(3, 3, "3/3"), (5, 4, "1/5"), (9, 0, "1/1"), (6, 21, "42/42")] {
        take_hits();
        let r = run(
            &src(&format!("procedure F(): Text begin exit(RoMeasure({kind}, {n})); end;")),
            "F",
        );
        assert_eq!(text(r), want, "kind {kind}, N {n}");
        assert!(
            take_hits().contains(&Assumption::SqlReadCostUnmeasuredShape),
            "kind {kind}, N {n}"
        );
    }
    for (body, want) in [
        ("RoReset(5); Mark(); Row.Get(1); Row.Get(2);", "2/2"),
        (
            "RoReset(5); Mark(); Row.SetRange(Name, 'row 1'); Cnt := Row.Count();",
            "1/1",
        ),
    ] {
        take_hits();
        let s = src(&format!(
            "procedure F(): Text var Row: Record Row; Cnt: Integer; begin {body} exit(Measured()); end;"
        ));
        assert_eq!(text(run(&s, "F")), want, "{body}");
        assert!(take_hits().contains(&Assumption::SqlReadCostUnmeasuredShape), "{body}");
    }
}

/// Nothing between two reads: the change is 0 (SqlCounters.al:105-119), a plain number.
#[test]
fn nothing_between_two_reads_is_a_change_of_zero() {
    let s = src("procedure F(): Text var X: Integer; begin Mark(); X := 1 + 2; exit(Measured()); end;");
    assert_eq!(text(run(&s, "F")), "0/0");
}

/// Session/lt-bl-sql-counters/SkSqlBudget.al (writers SK and SZ, al-runner issue 5861): a table whose Global Triggers setup
/// enables OnDatabaseInsert (Gt), a plain table (Plain) and the reporter's table of two key fields (Line), with subscribers to
/// the platform's Global Triggers (in the corpus: Base Application's GlobalTriggerManagement and the test's own).
const BUFFER_SRC: &str = r#"
table 50110 Gt { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50111 Plain { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50114 Line
{
    fields { field(1; "Group Code"; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Value; Text[50]) { } field(4; Weight; Integer) { } }
    keys { key(PK; "Group Code", "Line No.") { } }
}
codeunit 50112 Subs
{
    SingleInstance = true;
    var InsertsSeen: Integer;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId = Database::Gt then OnDatabaseInsert := true;
    end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure DbInsert(RecRef: RecordRef)
    begin
        if RecRef.Number() = Database::Gt then InsertsSeen += 1;
    end;
    procedure Seen(): Integer begin exit(InsertsSeen); end;
}
codeunit 50113 C
{
    var Stmts: BigInteger; Rows: BigInteger; Seq: Integer;
    local procedure Mark() begin Stmts := SessionInformation.SqlStatementsExecuted(); Rows := SessionInformation.SqlRowsRead(); end;
    local procedure Delta(): Text
    begin
        exit(Format(SessionInformation.SqlStatementsExecuted() - Stmts) + '/' + Format(SessionInformation.SqlRowsRead() - Rows));
    end;
    local procedure Stmt(): BigInteger begin exit(SessionInformation.SqlStatementsExecuted()); end;

    // SkSqlBudget.al:SZ_FirstInsert_TriggerSetupRunsAl.
    procedure FirstInsert(): Text
    var Subs: Codeunit Subs; Gt: Record Gt; Plain: Record Plain; GtFirst: BigInteger; PlainFirst: BigInteger; Acc: Text;
    begin
        Commit();
        Mark();
        Gt.Id := 1; Gt.Insert();
        GtFirst := Stmt() - Stmts;
        Mark(); Gt.Id := 2; Gt.Insert(); Acc := 'gt second ' + Delta() + '; ';
        Mark(); Gt.Id := 3; Gt.Insert(); Acc += 'gt third ' + Delta() + '; ';
        Mark(); Acc += 'gt count ' + Format(Gt.Count()) + ' ' + Delta() + '; ';
        Mark(); Plain.Id := 1; Plain.Insert();
        PlainFirst := Stmt() - Stmts;
        Mark(); Plain.Id := 2; Plain.Insert(); Acc += 'plain second ' + Delta() + '; ';
        Mark(); Plain.Id := 3; Plain.Insert(); Acc += 'plain third ' + Delta() + '; ';
        Mark(); Acc += 'plain count ' + Format(Plain.Count()) + ' ' + Delta();
        exit(Format(PlainFirst) + ' ' + Format(GtFirst > PlainFirst) + ' ' + Format(GtFirst >= 2) + ' ' + Format(Subs.Seen()) + ' | ' + Acc);
    end;

    // The size of the first Insert into a table whose setup enables OnDatabaseInsert is not pinned, only its lower bound.
    procedure GtFirstShown(): Text
    var Gt: Record Gt; GtFirst: BigInteger;
    begin
        Commit();
        Mark();
        Gt.Id := 1; Gt.Insert();
        GtFirst := Stmt() - Stmts;
        exit(Format(GtFirst));
    end;

    procedure GtFirstAtMost(): Boolean
    var Gt: Record Gt; GtFirst: BigInteger;
    begin
        Commit();
        Mark();
        Gt.Id := 1; Gt.Insert();
        GtFirst := Stmt() - Stmts;
        exit(GtFirst <= 1000);
    end;

    // SkSqlBudget.al:SZ_CopyLoop_WhileInsertsBuffered.
    local procedure Buffered(K: Integer; N: Integer): Text
    var Row: Record Plain; Temp: Record Plain temporary; i: Integer; Copy: Text;
    begin
        Row.Reset(); Row.DeleteAll(); Commit();
        for i := 1 to N do begin Temp.Id := i; Temp.Insert(); end;
        Mark();
        for i := 1 to K do begin Row.Id := 1000 + i; Row.Insert(); end;
        Copy := Delta();
        Mark();
        if Temp.FindSet() then repeat Row.Id := Temp.Id; Row.Insert(); until Temp.Next() = 0;
        Copy += ' ' + Delta();
        Mark();
        Row.SetRange(Id, 0, 2000);
        Copy += ' ' + Format(Row.Count()) + ' ' + Delta();
        exit(Copy);
    end;

    procedure WarmPlain()
    var Row: Record Plain;
    begin
        Mark(); Row.Id := 1; Row.Insert(); Row.DeleteAll(); Commit();
    end;

    procedure BufferedSeries(): Text
    begin
        WarmPlain();
        exit(Buffered(0, 10) + ' | ' + Buffered(1, 10) + ' | ' + Buffered(5, 10) + ' | ' + Buffered(5, 1) + ' | ' + Buffered(20, 20));
    end;

    local procedure FillBuffer(var TempLine: Record Line temporary; N: Integer)
    var i: Integer;
    begin
        TempLine.Reset(); TempLine.DeleteAll();
        for i := 1 to N do begin
            TempLine.Init(); TempLine."Group Code" := 'TEMP'; TempLine."Line No." := i; TempLine.Value := 'Value'; TempLine.Weight := i; TempLine.Insert();
        end;
    end;

    local procedure CopyLinesOnly(GroupCode: Code[20]; var TempLine: Record Line temporary)
    var Line: Record Line;
    begin
        if TempLine.FindSet() then
            repeat Line := TempLine; Line."Group Code" := GroupCode; Line.Insert(); until TempLine.Next() = 0;
    end;

    local procedure CopyLines(GroupCode: Code[20]; var TempLine: Record Line temporary): Integer
    var Line: Record Line;
    begin
        CopyLinesOnly(GroupCode, TempLine);
        Line.SetRange("Group Code", GroupCode);
        exit(Line.Count());
    end;

    // SkSqlBudget.al:SK_Repro_StatementBudget: the first Insert into the table in the session is inside the measurement.
    procedure ReproBudget(): Text
    var TempLine: Record Line temporary; Before: BigInteger; After: BigInteger; Executed: BigInteger; Copied: Integer;
    begin
        FillBuffer(TempLine, 10);
        Before := SessionInformation.SqlStatementsExecuted();
        Copied := CopyLines('BUDGET1', TempLine);
        After := SessionInformation.SqlStatementsExecuted();
        Executed := After - Before;
        exit(Format(Copied) + ' ' + Format(Executed <= 1000) + ' ' + StrSubstNo('SQL statements: %1, expected <= 1000', Executed));
    end;

    local procedure MeasureCopy(N: Integer; WithCommit: Boolean): Text
    var TempLine: Record Line temporary; Group: Code[20];
    begin
        FillBuffer(TempLine, N);
        Seq += 1;
        Group := 'SK' + Format(Seq);
        if WithCommit then Commit();
        Mark();
        CopyLinesOnly(Group, TempLine);
        exit(Delta());
    end;

    local procedure Repeat3(N: Integer; WithCommit: Boolean): Text
    begin
        exit(MeasureCopy(N, WithCommit) + ' ' + MeasureCopy(N, WithCommit) + ' ' + MeasureCopy(N, WithCommit));
    end;

    // SkSqlBudget.al:SK_Sizes_CopyOnly, after the table's first Insert.
    procedure CopyOnly(WithCommit: Boolean): Text
    var TempLine: Record Line temporary;
    begin
        Mark();
        FillBuffer(TempLine, 1);
        CopyLines('WARM', TempLine);
        exit(Repeat3(1, WithCommit) + ', ' + Repeat3(2, WithCommit) + ', ' + Repeat3(5, WithCommit) + ', ' + Repeat3(10, WithCommit) + ', ' + Repeat3(20, WithCommit));
    end;
}
"#;

/// SkSqlBudget.al:405-449 (SZ_FirstInsert_TriggerSetupRunsAl): the first Insert into a table adds one statement when its
/// Global Triggers setup enables nothing, and at least two when it enables OnDatabaseInsert; later Inserts are buffered, and
/// a Count sends the three buffered rows one by one.
#[test]
fn the_first_insert_into_a_table_and_the_buffered_ones_after_it() {
    assert_eq!(
        text(run_in(BUFFER_SRC, "C", "FirstInsert")),
        "1 Yes Yes 3 | gt second 0/0; gt third 0/0; gt count 3 4/1; plain second 0/0; plain third 0/0; plain count 3 4/1"
    );
}

/// Only the lower bound of the first Insert into a table whose setup enables OnDatabaseInsert is pinned (SkSqlBudget.al:400-402).
#[test]
fn the_first_insert_with_a_database_trigger_has_only_a_lower_bound() {
    refused(run_in(BUFFER_SRC, "C", "GtFirstShown"));
    refused(run_in(BUFFER_SRC, "C", "GtFirstAtMost"));
}

/// SkSqlBudget.al:476-499 (SZ_CopyLoop_WhileInsertsBuffered): every fifth buffered Insert sends one statement, also across the
/// copy loop; a Count sends what is left.
#[test]
fn a_copy_loop_while_inserts_are_buffered() {
    assert_eq!(
        text(run_in(BUFFER_SRC, "C", "BufferedSeries")),
        "0/0 2/0 10 1/1 | 0/0 2/0 11 2/1 | 1/0 2/0 15 1/1 | 1/0 0/0 6 2/1 | 4/0 4/0 40 1/1"
    );
}

/// SkSqlBudget.al:146-166 (SK_Repro_StatementBudget): the reporter's first test, with the table's first Insert in the
/// measurement.
#[test]
fn the_reporters_budget_with_the_first_insert() {
    assert_eq!(
        text(run_in(BUFFER_SRC, "C", "ReproBudget")),
        "10 Yes SQL statements: 4, expected <= 1000"
    );
}

/// SkSqlBudget.al:259-279 (SK_Sizes_CopyOnly): without a Commit, Inserts left buffered by one measurement count in the next.
#[test]
fn copies_without_a_count_carry_the_buffer_over() {
    assert_eq!(
        text(
            run_with(
                BUFFER_SRC,
                "C",
                "CopyOnly",
                vec![al_builtins::value::Value::Boolean(false)]
            )
            .map(|(v, _)| v)
        ),
        "0/0 0/0 0/0, 1/0 0/0 0/0, 1/0 1/0 1/0, 2/0 2/0 2/0, 4/0 4/0 4/0"
    );
    assert_eq!(
        text(
            run_with(
                BUFFER_SRC,
                "C",
                "CopyOnly",
                vec![al_builtins::value::Value::Boolean(true)]
            )
            .map(|(v, _)| v)
        ),
        "0/0 0/0 0/0, 0/0 0/0 0/0, 1/0 1/0 1/0, 2/0 2/0 2/0, 4/0 4/0 4/0"
    );
}
