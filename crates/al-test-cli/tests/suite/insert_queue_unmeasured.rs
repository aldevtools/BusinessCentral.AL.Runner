//! Shapes of the insert queue no corpus test measures (al-runner issues 8287 and 8288), refused until a probe measures them.
//! The queue: record/duplicate-insert/Y7407DuplicateInsertBuffer.al. A unique secondary key is pinned only on tables with a Blob
//! or AutoIncrement field (errors/last-error/InsertDuplicateKeyExcludeFlag.al:119, record/TestCalcFieldsTruncateUniqueContracts.al:335).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

/// One table per refused test, so that a refusal leaves no row another test reads.
const SRC: &str = r#"
table 51970 "IQ Sys Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 51971 "IQ Ref Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 51972 "IQ Query Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 51973 "IQ Flow Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 51974 "IQ Counter"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Rows; Integer) { FieldClass = FlowField; CalcFormula = count("IQ Flow Row"); }
        field(3; OtherRows; Integer) { FieldClass = FlowField; CalcFormula = count("IQ Other Row"); }
    }
    keys { key(PK; Code) { Clustered = true; } }
}
table 51975 "IQ Other Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 51976 "IQ Unique Row"
{
    fields { field(1; Code; Code[20]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Code) { Clustered = true; } key(ByName; Name) { Unique = true; } }
}
table 51977 "IQ Approx Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 51978 "IQ Kept Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 51979 "IQ Unique Temp"
{
    fields { field(1; Code; Code[20]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Code) { Clustered = true; } key(ByName; Name) { Unique = true; } }
}
table 51982 "IQ Xml Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 51983 "IQ Csv Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 51986 "IQ Blob" { fields { field(1; Code; Code[20]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { Clustered = true; } } }
query 51984 "IQ Xml Query"
{
    elements { dataitem(Q; "IQ Xml Row") { column(Code; Code) { } } }
}
query 51985 "IQ Csv Query"
{
    elements { dataitem(Q; "IQ Csv Row") { column(Code; Code) { } } }
}
query 51980 "IQ Query"
{
    elements { dataitem(Q; "IQ Query Row") { column(Code; Code) { } } }
}
query 51981 "IQ Other Query"
{
    elements { dataitem(Q; "IQ Other Row") { column(Code; Code) { } } }
}
codeunit 51982 "IQ Test"
{
    Subtype = Test;

    [Test]
    procedure GetBySystemIdWithDuplicatePending()
    var
        R: Record "IQ Sys Row";
        R2: Record "IQ Sys Row";
        R3: Record "IQ Sys Row";
    begin
        R.Code := 'A';
        R.Insert();
        R2.Code := 'A';
        R2.Insert();
        if not R3.GetBySystemId(R.SystemId) then
            Error('not found');
    end;

    [Test]
    procedure RecordRefGetBySystemIdWithDuplicatePending()
    var
        R: Record "IQ Ref Row";
        R2: Record "IQ Ref Row";
        RR: RecordRef;
    begin
        R.Code := 'A';
        R.Insert();
        R2.Code := 'A';
        R2.Insert();
        RR.Open(Database::"IQ Ref Row");
        if not RR.GetBySystemId(R.SystemId) then
            Error('not found');
    end;

    [Test]
    procedure CountApproxWithDuplicatePending()
    var
        R: Record "IQ Approx Row";
        R2: Record "IQ Approx Row";
    begin
        R.Code := 'A';
        R.Insert();
        R2.Code := 'A';
        R2.Insert();
        if R.CountApprox() < 0 then
            Error('negative');
    end;

    [Test]
    procedure QueryReadWithDuplicatePending()
    var
        R: Record "IQ Query Row";
        R2: Record "IQ Query Row";
        Q: Query "IQ Query";
    begin
        R.Code := 'A';
        R.Insert();
        R2.Code := 'A';
        R2.Insert();
        Q.Open();
        if not Q.Read() then
            Error('no row');
    end;

    [Test]
    procedure StaticQuerySaveAsXmlWithDuplicatePending()
    var
        R: Record "IQ Xml Row";
        R2: Record "IQ Xml Row";
        B: Record "IQ Blob" temporary;
        OS: OutStream;
    begin
        R.Code := 'A';
        R.Insert();
        R2.Code := 'A';
        R2.Insert();
        B.Data.CreateOutStream(OS);
        Query.SaveAsXml(Query::"IQ Xml Query", OS);
    end;

    [Test]
    procedure StaticQuerySaveAsCsvWithDuplicatePending()
    var
        R: Record "IQ Csv Row";
        R2: Record "IQ Csv Row";
        B: Record "IQ Blob" temporary;
        OS: OutStream;
    begin
        R.Code := 'A';
        R.Insert();
        R2.Code := 'A';
        R2.Insert();
        B.Data.CreateOutStream(OS);
        Query.SaveAsCsv(Query::"IQ Csv Query", OS);
    end;

    [Test]
    procedure KeptStaticQuerySaveAsXmlNothingPending()
    var
        R: Record "IQ Kept Row";
        B: Record "IQ Blob" temporary;
        OS: OutStream;
    begin
        R.Code := 'K';
        R.Insert();
        B.Data.CreateOutStream(OS);
        if not Query.SaveAsXml(Query::"IQ Xml Query", OS) then
            Error('not saved');
    end;

    [Test]
    procedure FlowFieldOfAnotherTableWithDuplicatePending()
    var
        R: Record "IQ Flow Row";
        R2: Record "IQ Flow Row";
        C: Record "IQ Counter";
    begin
        R.Code := 'A';
        R.Insert();
        R2.Code := 'A';
        R2.Insert();
        C.CalcFields(Rows);
        if C.Rows <> 1 then
            Error('%1', C.Rows);
    end;

    [Test]
    procedure UniqueSecondaryKeyDuplicate()
    var
        U: Record "IQ Unique Row";
    begin
        U.Code := 'A';
        U.Name := 'N';
        U.Insert();
        U.Code := 'B';
        asserterror U.Insert();
    end;

    // Neighbors that keep main's result.
    [Test]
    procedure KeptGetBySystemIdNothingPending()
    var
        R: Record "IQ Kept Row";
        R3: Record "IQ Kept Row";
    begin
        R.Code := 'A';
        R.Insert();
        if not R3.GetBySystemId(R.SystemId) then
            Error('not found');
    end;

    [Test]
    procedure KeptReadsOfOtherTablesWhileADuplicateIsPending()
    var
        R: Record "IQ Kept Row";
        R2: Record "IQ Kept Row";
        O: Record "IQ Other Row";
        C: Record "IQ Counter";
        Q: Query "IQ Other Query";
    begin
        O.Code := 'X';
        O.Insert();
        R.Code := 'A';
        R.Insert();
        R2.Code := 'A';
        R2.Insert();
        C.CalcFields(OtherRows);
        if C.OtherRows <> 1 then
            Error('%1', C.OtherRows);
        Q.Open();
        if not Q.Read() then
            Error('no row');
        if O.GetBySystemId(O.SystemId) then;
        asserterror R.Count();
    end;

    [Test]
    procedure KeptUniqueSecondaryKeyOfATemporaryRecord()
    var
        U: Record "IQ Unique Temp" temporary;
    begin
        U.Code := 'A';
        U.Name := 'N';
        U.Insert();
        U.Code := 'B';
        asserterror U.Insert();
    end;

    [Test]
    procedure KeptUniqueSecondaryKeyWithTheReturnValue()
    var
        U: Record "IQ Unique Temp";
    begin
        U.Code := 'A';
        U.Name := 'N';
        U.Insert();
        U.Code := 'B';
        if U.Insert() then
            Error('inserted');
    end;
}
"#;

fn of(out: &[(String, Outcome)], n: &str) -> Outcome {
    out.iter()
        .find(|(t, _)| t == n)
        .map(|(_, o)| o.clone())
        .unwrap_or_else(|| panic!("no outcome for {n}: {out:?}"))
}

fn refused_with(out: &[(String, Outcome)], n: &str, part: &str) {
    match of(out, n) {
        Outcome::NotRun(r) => assert!(r.contains(part), "{n}: {r}"),
        other => panic!("{n}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn issue_8288_reads_that_no_case_measures_with_a_duplicate_pending_are_refused() {
    let out = outcomes_in(SRC, "IQ Test");
    for n in [
        "GetBySystemIdWithDuplicatePending",
        "RecordRefGetBySystemIdWithDuplicatePending",
        "CountApproxWithDuplicatePending",
        "QueryReadWithDuplicatePending",
        "FlowFieldOfAnotherTableWithDuplicatePending",
    ] {
        refused_with(&out, n, "while the error of an Insert into");
    }
}

/// The static Query.SaveAsXml and Query.SaveAsCsv read the query's tables as the instance methods do.
#[test]
fn issue_8288_static_query_save_as_with_a_duplicate_pending_is_refused() {
    let out = outcomes_in(SRC, "IQ Test");
    for n in [
        "StaticQuerySaveAsXmlWithDuplicatePending",
        "StaticQuerySaveAsCsvWithDuplicatePending",
    ] {
        refused_with(&out, n, "while the error of an Insert into");
    }
    assert_eq!(of(&out, "KeptStaticQuerySaveAsXmlNothingPending"), Outcome::Pass);
}

#[test]
fn issue_8287_a_unique_secondary_key_duplicate_on_a_queued_table_is_refused() {
    let out = outcomes_in(SRC, "IQ Test");
    refused_with(&out, "UniqueSecondaryKeyDuplicate", "unique secondary key");
}

#[test]
fn reads_and_inserts_outside_the_unmeasured_shapes_keep_main_s_result() {
    let out = outcomes_in(SRC, "IQ Test");
    for n in [
        "KeptGetBySystemIdNothingPending",
        "KeptReadsOfOtherTablesWhileADuplicateIsPending",
        "KeptUniqueSecondaryKeyOfATemporaryRecord",
        "KeptUniqueSecondaryKeyWithTheReturnValue",
    ] {
        assert_eq!(of(&out, n), Outcome::Pass, "{n}");
    }
}
