//! Query objects end to end: AL source → compile → run on the in-memory store.
//! Each test cites the corpus file that pins the behaviour
//! (corpus/tests/al-language/query/...).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

/// corpus _fixtures/tables ALT Universal (the fields the query fixtures read).
const TABLES: &str = r#"
table 60000 "ALT Universal"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Integer Field"; Integer) { }
        field(3; "Text Field"; Text[100]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 60861 "QJ Customer"
{
    fields { field(1; "No."; Code[20]) { } field(2; Name; Text[50]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 60862 "QJ Order"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Customer No."; Code[20]) { }
        field(3; Amount; Decimal) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 60870 "QX Num"
{
    fields { field(1; "No."; Integer) { } field(2; Big; BigInteger) { } field(3; Dec; Decimal) { } field(4; Grp; Code[10]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
query 60871 "QX Reverse Sign Types"
{
    QueryType = Normal;
    OrderBy = ascending(No);
    elements
    {
        dataitem(Num; "QX Num")
        {
            column(No; "No.") { }
            column(NoReversed; "No.") { ReverseSign = true; }
            column(BigReversed; Big) { ReverseSign = true; }
            column(DecReversed; Dec) { ReverseSign = true; }
        }
    }
}
query 60872 "QX Reverse Sign Total"
{
    QueryType = Normal;
    OrderBy = ascending(Grp);
    elements
    {
        dataitem(Num; "QX Num")
        {
            column(Grp; Grp) { }
            column(DecTotal; Dec) { Method = Sum; ReverseSign = true; }
        }
    }
}
query 60874 "QX Tie Groups"
{
    QueryType = Normal;
    OrderBy = ascending(Grp);
    elements
    {
        dataitem(Num; "QX Num")
        {
            column(Grp; Grp) { }
            column(Big; Big) { }
            column(DecTotal; Dec) { Method = Sum; }
        }
    }
}
query 60873 "QX Min Max"
{
    QueryType = Normal;
    OrderBy = ascending(Grp);
    elements
    {
        dataitem(Num; "QX Num")
        {
            column(Grp; Grp) { }
            column(Lowest; Dec) { Method = Min; }
            column(Highest; Dec) { Method = Max; }
        }
    }
}
query 60022 "ALT Universal Query"
{
    QueryType = Normal;
    OrderBy = ascending(EntryNo);
    elements
    {
        dataitem(Universal; "ALT Universal")
        {
            column(EntryNo; "Entry No.") { Caption = 'Entry No.'; }
            column(IntegerValue; "Integer Field") { Caption = 'Integer Value'; }
            column(TextValue; "Text Field") { Caption = 'Text Value'; }
        }
    }
}
query 60863 "QJ Cust Orders Inner"
{
    QueryType = Normal;
    OrderBy = ascending(EntryNo);
    elements
    {
        dataitem(Customer; "QJ Customer")
        {
            column(CustNo; "No.") { }
            column(CustName; "Name") { }
            dataitem(Ord; "QJ Order")
            {
                DataItemLink = "Customer No." = Customer."No.";
                SqlJoinType = InnerJoin;
                column(EntryNo; "Entry No.") { }
                column(Amount; "Amount") { }
            }
        }
    }
}
query 60864 "QJ Cust Orders Left"
{
    QueryType = Normal;
    OrderBy = ascending(CustNo);
    elements
    {
        dataitem(Customer; "QJ Customer")
        {
            column(CustNo; "No.") { }
            column(CustName; "Name") { }
            dataitem(Ord; "QJ Order")
            {
                DataItemLink = "Customer No." = Customer."No.";
                SqlJoinType = LeftOuterJoin;
                column(EntryNo; "Entry No.") { }
                column(Amount; "Amount") { }
            }
        }
    }
}
query 60601 "QJ Cust Orders Implicit"
{
    QueryType = Normal;
    OrderBy = ascending(CustNo);
    elements
    {
        dataitem(Customer; "QJ Customer")
        {
            column(CustNo; "No.") { }
            dataitem(Ord; "QJ Order")
            {
                DataItemLink = "Customer No." = Customer."No.";
                column(EntryNo; "Entry No.") { }
            }
        }
    }
}
query 60760 "QJ Order Sum"
{
    QueryType = Normal;
    OrderBy = ascending(CustNo);
    elements
    {
        dataitem(Order; "QJ Order")
        {
            column(CustNo; "Customer No.") { }
            column(TotalAmount; Amount) { Method = Sum; }
            column(CountAmount) { Method = Count; }
            column(AverageAmount; Amount) { Method = Average; }
            column(MinAmount; Amount) { Method = Min; }
            column(MaxAmount; Amount) { Method = Max; }
        }
    }
}
query 60761 "QJ Order Scalar Sum"
{
    QueryType = Normal;
    elements
    {
        dataitem(Order; "QJ Order")
        {
            column(TotalAmount; Amount) { Method = Sum; }
            column(CountAmount) { Method = Count; }
        }
    }
}
query 60768 "QJ Order Sum Filtered"
{
    QueryType = Normal;
    OrderBy = ascending(CustNo);
    elements
    {
        dataitem(Order; "QJ Order")
        {
            column(CustNo; "Customer No.") { }
            column(TotalAmount; Amount)
            {
                ColumnFilter = TotalAmount = filter(> 0);
                Method = Sum;
            }
        }
    }
}
query 60769 "QJ Order Filtered"
{
    QueryType = Normal;
    OrderBy = ascending(EntryNo);
    elements
    {
        dataitem(Order; "QJ Order")
        {
            column(EntryNo; "Entry No.") { }
            column(CustNo; "Customer No.")
            {
                ColumnFilter = CustNo = const('C1');
            }
            column(Amount; Amount) { }
        }
    }
}
query 60935 "QJ Order Reverse Sign"
{
    QueryType = Normal;
    OrderBy = ascending(EntryNo);
    elements
    {
        dataitem(Order; "QJ Order")
        {
            column(EntryNo; "Entry No.") { }
            column(Amount; Amount) { }
            column(ReversedAmount; Amount) { ReverseSign = true; }
        }
    }
}
query 68534 "QRO Descending"
{
    QueryType = Normal;
    OrderBy = descending(EntryNo);
    elements
    {
        dataitem(Entry; "QJ Order")
        {
            column(EntryNo; "Entry No.") { }
            column(Amount; Amount) { }
        }
    }
}
query 68537 "QRO Descending Amount"
{
    QueryType = Normal;
    OrderBy = descending(Amount);
    elements
    {
        dataitem(Entry; "QJ Order")
        {
            column(EntryNo; "Entry No.") { }
            column(Amount; Amount) { }
        }
    }
}
query 68535 "QRO By Amount"
{
    QueryType = Normal;
    OrderBy = ascending(Amount);
    elements
    {
        dataitem(Entry; "QJ Order")
        {
            column(EntryNo; "Entry No.") { }
            column(Amount; Amount) { }
        }
    }
}
query 68650 "QTP Top Two"
{
    QueryType = Normal;
    OrderBy = descending(EntryNo);
    TopNumberOfRows = 2;
    elements
    {
        dataitem(Entry; "QJ Order")
        {
            column(EntryNo; "Entry No.") { }
        }
    }
}
query 68651 "QTP Top Two Unordered"
{
    QueryType = Normal;
    TopNumberOfRows = 2;
    elements
    {
        dataitem(Entry; "QJ Order")
        {
            column(EntryNo; "Entry No.") { }
        }
    }
}
query 68531 "QRW Entries Unordered"
{
    QueryType = Normal;
    elements
    {
        dataitem(Entry; "QJ Order")
        {
            column(EntryNo; "Entry No.") { }
            column(Amount; Amount) { }
        }
    }
}
table 60497 "QDTF Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Status; Option) { OptionMembers = Open,Closed,Held; }
        field(3; Amount; Decimal) { }
        field(4; Kind; Integer) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}
query 60500 "QDTF Open Rows"
{
    QueryType = Normal;
    OrderBy = ascending(RowCode);
    elements
    {
        dataitem(Row; "QDTF Row")
        {
            DataItemTableFilter = Status = const(Open);
            column(RowCode; "Code") { }
            column(RowAmount; Amount) { }
        }
    }
}
query 60501 "QDTF Two Field Filter"
{
    QueryType = Normal;
    OrderBy = ascending(RowCode);
    elements
    {
        dataitem(Row; "QDTF Row")
        {
            DataItemTableFilter = Status = const(Open), Amount = filter(> 100);
            column(RowCode; "Code") { }
        }
    }
}
query 60507 "QDTF Status Filtered"
{
    QueryType = Normal;
    OrderBy = ascending(RowCode);
    elements
    {
        dataitem(Row; "QDTF Row")
        {
            DataItemTableFilter = Status = const(Open);
            column(RowCode; "Code") { }
            filter(RowStatus; Status) { }
        }
    }
}
query 69980 "QDB Kind Filtered"
{
    QueryType = Normal;
    elements
    {
        dataitem(Row; "QDTF Row")
        {
            DataItemTableFilter = Kind = const(Database::"QDTF Row");
            column(RowCode; Code) { }
        }
    }
}
table 68600 "QFG Header"
{
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 68601 "QFG Entry"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Header No."; Code[20]) { }
        field(3; "Posting Date"; Date) { }
        field(4; Amount; Decimal) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
query 68600 "QFG Join Totals"
{
    QueryType = Normal;
    OrderBy = ascending(HeaderNo);
    elements
    {
        dataitem(Header; "QFG Header")
        {
            column(HeaderNo; "No.") { }
            dataitem(Entry; "QFG Entry")
            {
                DataItemLink = "Header No." = Header."No.";
                SqlJoinType = InnerJoin;
                column(TotalAmount; Amount) { Method = Sum; }
                column(EntryCount) { Method = Count; }
                filter(DateFilter; "Posting Date") { }
            }
        }
    }
}
table 60498 "QDTF Header"
{
    fields { field(1; "No."; Code[20]) { } field(2; "Variant Code"; Code[20]) { } field(3; Descr; Text[50]) { } }
    keys { key(PK; "No.", "Variant Code") { Clustered = true; } }
}
table 60499 "QDTF Line"
{
    fields
    {
        field(1; "Header No."; Code[20]) { }
        field(2; "Variant Code"; Code[20]) { }
        field(3; "Line No."; Integer) { }
        field(4; Tag; Text[50]) { }
    }
    keys { key(PK; "Header No.", "Variant Code", "Line No.") { Clustered = true; } }
}
query 60502 "QDTF Composite Link"
{
    QueryType = Normal;
    OrderBy = ascending(HdrDescr);
    elements
    {
        dataitem(Hdr; "QDTF Header")
        {
            column(HdrDescr; Descr) { }
            dataitem(Ln; "QDTF Line")
            {
                DataItemLink = "Header No." = Hdr."No.", "Variant Code" = Hdr."Variant Code";
                SqlJoinType = InnerJoin;
                column(LineTag; Tag) { }
            }
        }
    }
}
table 60974 "QFF Line"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Header No."; Code[20]) { }
        field(3; "Posting Date"; Date) { }
        field(4; Amount; Decimal) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 60975 "QFF Header"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Date Filter"; Date) { FieldClass = FlowFilter; }
        field(3; "Total Amount"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("QFF Line".Amount where("Header No." = field("No.")));
        }
        field(4; "Dated Amount"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("QFF Line".Amount where("Header No." = field("No."), "Posting Date" = field("Date Filter")));
        }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
query 60271 "QFF Header Dated"
{
    QueryType = Normal;
    elements
    {
        dataitem(QffHeader; "QFF Header")
        {
            column(No; "No.") { }
            filter(DateFilter; "Date Filter") { }
            column(DatedAmount; "Dated Amount") { }
        }
    }
}
query 60272 "QFF Header Sum"
{
    QueryType = Normal;
    elements
    {
        dataitem(QffHeader; "QFF Header")
        {
            column(Total; "Total Amount") { Method = Sum; }
        }
    }
}
query 68532 "QRW Amounts"
{
    QueryType = Normal;
    elements
    {
        dataitem(Entry; "QJ Order")
        {
            column(Amount; Amount) { }
        }
    }
}
table 60990 "QC Row"
{
    fields { field(1; Id; Integer) { } field(2; Kind; Integer) { } field(3; Amt; Decimal) { } }
    keys { key(PK; Id) { } }
}
table 60991 "QC Log"
{
    fields { field(1; Name; Text[30]) { } field(2; Val; Integer) { } }
    keys { key(PK; Name) { } }
}
// The shape of Base App's query 522 (a filter() element, procedures that set it) and of the
// fixture ALTManualQueryIntegrationEvent (a procedure that raises its own IntegrationEvent).
query 60992 "QC With Code"
{
    QueryType = Normal;
    elements
    {
        dataitem(Row; "QC Row")
        {
            filter(KindF; Kind) { }
            column(Total; Amt) { Method = Sum; }
        }
    }

    procedure SetKind(K: Integer; Row2: Record "QC Row")
    begin
        SetRange(KindF, K);
        if Row2.Id <> 0 then
            SetRange(KindF, Row2.Kind);
    end;

    procedure Raise(V: Integer)
    var L: Record "QC Log";
    begin
        L.Name := 'ran'; L.Val := V; L.Insert();
        OnRaised(V);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnRaised(V: Integer)
    begin
    end;

    trigger OnBeforeOpen()
    begin
    end;
}
codeunit 60993 "QC Sub"
{
    [EventSubscriber(ObjectType::Query, Query::"QC With Code", 'OnRaised', '', false, false)]
    local procedure OnRaised(V: Integer)
    var L: Record "QC Log";
    begin
        L.Name := 'fired'; L.Val := V; L.Insert();
    end;
}
query 60996 "QC Unordered Groups"
{
    elements { dataitem(Row; "QC Row") { column(Kind; Kind) { } column(Total; Amt) { Method = Sum; } } }
}
query 60997 "QC Cross"
{
    elements
    {
        dataitem(A; "QC Row")
        {
            DataItemTableFilter = Id = filter('>=2');
            column(Id; Id) { }
            dataitem(B; "QC Log")
            {
                SqlJoinType = CrossJoin;
                column(Name; Name) { }
            }
        }
    }
}
query 60998 "QC Cross Sum"
{
    elements
    {
        dataitem(A; "QC Row")
        {
            column(Id; Id) { }
            dataitem(B; "QC Row")
            {
                SqlJoinType = CrossJoin;
                column(SumAmt; Amt) { Method = Sum; }
            }
        }
    }
}
query 60999 "QC Cross Linked"
{
    elements
    {
        dataitem(A; "QC Row")
        {
            column(Id; Id) { }
            dataitem(B; "QC Row")
            {
                SqlJoinType = CrossJoin;
                DataItemLink = Id = A.Id;
                column(Amt; Amt) { }
            }
        }
    }
}
query 60994 "QC With Variable"
{
    elements { dataitem(Row; "QC Row") { column(Id; Id) { } } }
    var
        OpenCount: Integer;
    trigger OnBeforeOpen()
    var L: Record "QC Log";
    begin
        OpenCount += 1;
        L.Name := Format(L.Count()) + 'open' + Format(OpenCount) + '[' + CurrQuery.GetFilters() + ']'; L.Insert();
        if OpenCount = 2 then
            CurrQuery.SetRange(Id, 3);
    end;
    procedure Opens(): Integer begin exit(OpenCount); end;
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!("{TABLES}\ncodeunit 50100 C {{ {body} }}");
    al_testkit::run_in(&src, "C", "F")
}

fn unsupported(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(s)) => s,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Entries 1 and 2 of the ALT Universal table, as TestQueryObject.al:InsertQueryRows.
const SEED_UNIVERSAL: &str = "
    procedure SeedU()
    var R: Record \"ALT Universal\";
    begin
        R.\"Entry No.\" := 1; R.\"Integer Field\" := 10; R.\"Text Field\" := 'First'; R.Insert();
        R.\"Entry No.\" := 2; R.\"Integer Field\" := 20; R.\"Text Field\" := 'Second'; R.Insert();
    end;
";

/// Customers C1 Alice, C2 Bob, C3 Carol; orders 1 C1 100, 2 C1 200, 3 C2 300
/// (query/TestQueryJoin.al:44-53).
const SEED_JOIN: &str = "
    procedure SeedJ()
    var C: Record \"QJ Customer\"; O: Record \"QJ Order\";
    begin
        C.\"No.\" := 'C1'; C.Name := 'Alice'; C.Insert();
        C.\"No.\" := 'C2'; C.Name := 'Bob'; C.Insert();
        C.\"No.\" := 'C3'; C.Name := 'Carol'; C.Insert();
        O.\"Entry No.\" := 1; O.\"Customer No.\" := 'C1'; O.Amount := 100; O.Insert();
        O.\"Entry No.\" := 2; O.\"Customer No.\" := 'C1'; O.Amount := 200; O.Insert();
        O.\"Entry No.\" := 3; O.\"Customer No.\" := 'C2'; O.Amount := 300; O.Insert();
    end;
";

fn with(seed: &str, body: &str) -> String {
    format!("{SEED_UNIVERSAL}{SEED_JOIN}{seed}{body}")
}

// query/TestQueryObject.al:15-40 Query_Open_Read_ReturnsInsertedRows.
#[test]
fn open_read_returns_the_rows_in_order_by_order() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"ALT Universal Query\"; Seen: Text;
        begin
            SeedU();
            Q.Open();
            while Q.Read() do
                Seen += StrSubstNo('%1/%2/%3;', Q.EntryNo, Q.IntegerValue, Q.TextValue);
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "1/10/First;2/20/Second;");
}

// query/TestQueryObject.al:42-60 Query_Close_Reopen_RestartsDatasetFromFirstRow.
#[test]
fn close_and_reopen_restarts_from_the_first_row() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"ALT Universal Query\"; Seen: Text;
        begin
            SeedU();
            Q.Open();
            if Q.Read() then Seen += Format(Q.EntryNo);
            Q.Close();
            Q.Open();
            if Q.Read() then Seen += Format(Q.EntryNo);
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "11");
}

// query/TestQueryObject.al:62-80 SetRange, :82-100 SetFilter with a placeholder, :102-120 wildcard.
#[test]
fn runtime_filters_on_columns() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"ALT Universal Query\"; Seen: Text;
        begin
            SeedU();
            Q.SetRange(EntryNo, 2);
            Q.Open(); while Q.Read() do Seen += 'R' + Format(Q.EntryNo); Q.Close();
            Q.SetRange(EntryNo);
            Q.SetFilter(IntegerValue, '>%1', 10);
            Q.Open(); while Q.Read() do Seen += 'P' + Format(Q.EntryNo); Q.Close();
            Q.SetRange(IntegerValue);
            Q.SetFilter(TextValue, 'S*');
            Q.Open(); while Q.Read() do Seen += 'W' + Format(Q.EntryNo); Q.Close();
            Q.SetFilter(TextValue, 'Z*');
            Q.Open(); while Q.Read() do Seen += 'Z' + Format(Q.EntryNo); Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "R2P2W2");
}

// query/TestQueryObject.al:200-225 inclusive ranges and :270-300 alternated ranges.
#[test]
fn range_filters_include_both_ends() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"ALT Universal Query\"; Seen: Text;
        begin
            SeedU();
            Q.SetFilter(IntegerValue, '%1..%2', 10, 20);
            Q.Open(); while Q.Read() do Seen += Format(Q.EntryNo); Q.Close();
            Q.SetFilter(IntegerValue, '%1..%2|%3..%4', 5, 12, 30, 40);
            Q.Open(); while Q.Read() do Seen += '/' + Format(Q.EntryNo); Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "12/1");
}

// query/TestQueryObject.al:313-335 GetFilter after SetRange; :337-345 GetFilters.
#[test]
fn get_filter_reads_back_the_filter() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"ALT Universal Query\";
        begin
            Q.SetRange(EntryNo, 2);
            exit(Q.GetFilter(EntryNo));
        end;",
    ));
    assert_eq!(text(r), "2");
}

// query/TestQueryObject.al:347-365 ColumnName, ColumnCaption, ColumnNo.
#[test]
fn column_metadata() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"ALT Universal Query\";
        begin
            exit(Q.ColumnName(EntryNo) + '|' + Q.ColumnCaption(TextValue) + '|' +
                Format(Q.ColumnNo(EntryNo) > 0) + Format(Q.ColumnNo(EntryNo) <> Q.ColumnNo(IntegerValue)));
        end;",
    ));
    assert_eq!(text(r), "EntryNo|Text Value|YesYes");
}

// ColumnNo is |FNV-1a 32 of the column name in UTF-16LE| read as a signed 32-bit number
// (misc/lt-probes-needed-1/PnAQueryFiltersColumnNo.al:143-150, :168-175: twelve names checked).
#[test]
fn column_no_is_the_absolute_fnv1a_hash_of_the_column_name() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"ALT Universal Query\";
        begin
            exit(Format(Q.ColumnNo(EntryNo)) + '|' + Format(Q.ColumnNo(IntegerValue)) + '|' + Format(Q.ColumnNo(TextValue)));
        end;",
    ));
    assert_eq!(
        text(r),
        format!("{}|{}|{}", fnv("EntryNo"), fnv("IntegerValue"), fnv("TextValue"))
    );
}

fn fnv(name: &str) -> i32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in name.encode_utf16().flat_map(u16::to_le_bytes) {
        h = (h ^ u32::from(b)).wrapping_mul(0x0100_0193);
    }
    (h as i32).abs()
}

#[test]
fn column_no_reference_hash_matches_the_corpus_values() {
    // PnAQueryFiltersColumnNo.al:143-146.
    assert_eq!(fnv("Entry"), 1327805081);
    assert_eq!(fnv("Qty"), 352015135);
    assert_eq!(fnv("Amt"), 84079987);
}

// query/TestQueryJoin.al:79-115 InnerJoin drops the unmatched parent.
#[test]
fn inner_join_pairs_parent_with_child_and_drops_the_unmatched_parent() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QJ Cust Orders Inner\"; Seen: Text;
        begin
            SeedJ();
            Q.Open();
            while Q.Read() do
                Seen += StrSubstNo('%1/%2/%3/%4;', Q.CustNo, Q.CustName, Q.EntryNo, Q.Amount);
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "C1/Alice/1/100;C1/Alice/2/200;C2/Bob/3/300;");
}

// query/TestQueryJoin.al:117-145 LeftOuterJoin keeps the unmatched parent with default child columns;
// query/TestQueryJoinDefaultType.al:69-95 an omitted SqlJoinType joins as LeftOuterJoin.
#[test]
fn left_outer_join_keeps_the_unmatched_parent_and_is_the_default() {
    let body = |q: &str| {
        with(
            "",
            &format!(
                "procedure F(): Text
                var Q: Query \"{q}\"; Seen: Text;
                begin
                    SeedJ();
                    Q.Open();
                    while Q.Read() do
                        Seen += StrSubstNo('%1/%2;', Q.CustNo, Q.EntryNo);
                    Q.Close();
                    exit(Seen);
                end;"
            ),
        )
    };
    assert_eq!(text(run(&body("QJ Cust Orders Left"))), "C1/1;C1/2;C2/3;C3/0;");
    assert_eq!(text(run(&body("QJ Cust Orders Implicit"))), "C1/1;C1/2;C2/3;C3/0;");
}

// query/TestQueryJoin.al:147-175 runtime filter on a parent column, :177-205 on a child column.
#[test]
fn runtime_filters_reach_parent_and_child_columns_of_a_join() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QJ Cust Orders Inner\"; Seen: Text;
        begin
            SeedJ();
            Q.SetRange(CustNo, 'C2');
            Q.Open(); while Q.Read() do Seen += Format(Q.EntryNo); Q.Close();
            Q.SetRange(CustNo);
            Q.SetFilter(Amount, '>%1', 150);
            Q.Open(); while Q.Read() do Seen += '/' + Format(Q.EntryNo); Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "3/2/3");
}

// query/TestQueryAggregation.al:36-80 each Method per group; :82-100 one row per group.
#[test]
fn aggregates_group_by_the_other_columns() {
    let r = run(&with(
        "procedure SeedO()
         var O: Record \"QJ Order\";
         begin
            O.\"Entry No.\" := 1; O.\"Customer No.\" := 'C1'; O.Amount := 100; O.Insert();
            O.\"Entry No.\" := 2; O.\"Customer No.\" := 'C1'; O.Amount := 200; O.Insert();
            O.\"Entry No.\" := 3; O.\"Customer No.\" := 'C2'; O.Amount := 50; O.Insert();
         end;",
        "procedure F(): Text
        var Q: Query \"QJ Order Sum\"; Seen: Text;
        begin
            SeedO();
            Q.Open();
            while Q.Read() do
                Seen += StrSubstNo('%1:%2,%3,%4,%5,%6;', Q.CustNo, Q.TotalAmount, Q.CountAmount, Q.AverageAmount, Q.MinAmount, Q.MaxAmount);
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "C1:300,2,150,100,200;C2:50,1,50,50,50;");
}

// query/TestQueryAggregation.al:102-135 a scalar aggregate is one row; :137-160 even over an empty
// table; :162-180 a grouped query over an empty table is empty.
#[test]
fn scalar_aggregate_is_always_one_row() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QJ Order Scalar Sum\"; G: Query \"QJ Order Sum\"; Seen: Text;
        begin
            Q.Open();
            while Q.Read() do Seen += StrSubstNo('%1,%2;', Q.TotalAmount, Q.CountAmount);
            Q.Close();
            G.Open();
            while G.Read() do Seen += 'G';
            G.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "0,0;");
}

// query/TestQueryAggregation.al:182-215 a filter on an aggregated column is evaluated per group.
#[test]
fn filter_on_an_aggregated_column_is_a_having_filter() {
    let r = run(&with(
        "procedure SeedO()
         var O: Record \"QJ Order\";
         begin
            O.\"Entry No.\" := 1; O.\"Customer No.\" := 'C1'; O.Amount := 60; O.Insert();
            O.\"Entry No.\" := 2; O.\"Customer No.\" := 'C1'; O.Amount := 60; O.Insert();
            O.\"Entry No.\" := 3; O.\"Customer No.\" := 'C2'; O.Amount := 100; O.Insert();
         end;",
        "procedure F(): Text
        var Q: Query \"QJ Order Sum\"; Seen: Text;
        begin
            SeedO();
            Q.SetFilter(TotalAmount, '>%1', 100);
            Q.Open();
            while Q.Read() do Seen += Q.CustNo + Format(Q.TotalAmount);
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "C1120");
}

// query/TestQueryColumnFilter.al:36-60 a static ColumnFilter on a Sum column drops groups;
// :62-100 a runtime filter on the same column replaces it; :102-148 on a plain column it drops rows.
#[test]
fn static_column_filters() {
    let r = run(&with(
        "procedure SeedO()
         var O: Record \"QJ Order\";
         begin
            O.\"Entry No.\" := 1; O.\"Customer No.\" := 'C1'; O.Amount := 100; O.Insert();
            O.\"Entry No.\" := 2; O.\"Customer No.\" := 'C1'; O.Amount := -100; O.Insert();
            O.\"Entry No.\" := 3; O.\"Customer No.\" := 'C2'; O.Amount := 50; O.Insert();
         end;",
        "procedure F(): Text
        var S: Query \"QJ Order Sum Filtered\"; P: Query \"QJ Order Filtered\"; Seen: Text;
        begin
            SeedO();
            S.Open(); while S.Read() do Seen += S.CustNo + ':' + Format(S.TotalAmount) + ';'; S.Close();
            S.SetFilter(TotalAmount, '<%1', 10);
            S.Open(); while S.Read() do Seen += S.CustNo + ':' + Format(S.TotalAmount) + ';'; S.Close();
            P.Open(); while P.Read() do Seen += Format(P.EntryNo) + P.CustNo + ';'; P.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "C2:50;C1:0;1C1;2C1;");
}

// query/TestQueryReverseSign.al:36-125.
#[test]
fn reverse_sign_negates_one_column_only() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QJ Order Reverse Sign\"; O: Record \"QJ Order\"; Seen: Text;
        begin
            O.\"Entry No.\" := 1; O.\"Customer No.\" := 'C1'; O.Amount := 100; O.Insert();
            Q.Open(); while Q.Read() do Seen += Format(Q.Amount) + '/' + Format(Q.ReversedAmount); Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "100/-100");
}

// query/TestQueryOrderAfterWrite.al:60-110 descending and non-key OrderBy, ties by the primary key.
#[test]
fn order_by_descending_and_by_a_non_key_column() {
    let seed = "procedure SeedO()
         var O: Record \"QJ Order\";
         begin
            O.\"Entry No.\" := 1; O.Amount := 10; O.Insert();
            O.\"Entry No.\" := 2; O.Amount := 10; O.Insert();
            O.\"Entry No.\" := 3; O.Amount := 30; O.Insert();
            O.\"Entry No.\" := 4; O.Amount := 10; O.Insert();
         end;";
    let r = run(&with(
        seed,
        "procedure F(): Text
        var D: Query \"QRO Descending\"; A: Query \"QRO By Amount\"; Seen: Text;
        begin
            SeedO();
            D.Open(); while D.Read() do Seen += Format(D.EntryNo) + ';'; D.Close();
            Seen += '|';
            A.Open(); while A.Read() do Seen += Format(A.EntryNo) + ';'; A.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "4;3;2;1;|1;2;4;3;");
}

// query/TestQueryTopProperty.al:56-181 the property is the initial cap, the method replaces it.
#[test]
fn top_number_of_rows_property_and_method() {
    let seed = "procedure SeedO()
         var O: Record \"QJ Order\";
         begin
            O.\"Entry No.\" := 1; O.Insert();
            O.\"Entry No.\" := 2; O.Insert();
            O.\"Entry No.\" := 3; O.Insert();
            O.\"Entry No.\" := 4; O.Insert();
         end;";
    let r = run(&with(
        seed,
        "procedure F(): Text
        var Q: Query \"QTP Top Two\"; U: Query \"QTP Top Two Unordered\"; Seen: Text;
        begin
            SeedO();
            Seen += Format(Q.TopNumberOfRows()) + ':';
            Q.Open(); while Q.Read() do Seen += Format(Q.EntryNo) + ';'; Q.Close();
            Q.TopNumberOfRows(3);
            Q.Open(); while Q.Read() do Seen += Format(Q.EntryNo) + ';'; Q.Close();
            Q.TopNumberOfRows(0);
            Q.Open(); while Q.Read() do Seen += Format(Q.EntryNo) + ';'; Q.Close();
            U.Open(); while U.Read() do Seen += 'u'; U.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "2:4;3;4;3;2;4;3;2;1;uu");
}

// query/TestQueryTopProperty.al:173-181 a negative cap is an error and leaves the property in place.
#[test]
fn negative_top_number_of_rows_is_an_error() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QTP Top Two\";
        begin
            asserterror Q.TopNumberOfRows(-1);
            exit(GetLastErrorText() + '|' + Format(Q.TopNumberOfRows()));
        end;",
    ));
    assert_eq!(text(r), "Top number of rows cannot be negative|2");
}

// query/TestQueryReadAfterWrite.al:102-118 a Modify in the loop resumes after the last row,
// :120-141 an insert after the position is read and one before it is not, :143-162 a deleted row is skipped.
#[test]
fn read_resumes_after_the_last_row_when_the_table_changes() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QRW Entries Unordered\"; O: Record \"QJ Order\"; Seen: Text;
        begin
            O.\"Entry No.\" := 2; O.Insert();
            O.\"Entry No.\" := 3; O.Insert();
            Q.Open();
            while Q.Read() do begin
                Seen += Format(Q.EntryNo) + ',';
                if Q.EntryNo = 2 then begin
                    O.\"Entry No.\" := 1; O.Insert();
                    O.\"Entry No.\" := 4; O.Insert();
                    O.Get(3); O.Delete();
                end;
            end;
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "2,4,");
}

// query/TestQueryOrderAfterWrite.al:115-135 a write in the loop must not extend a TopNumberOfRows cap.
#[test]
fn a_write_in_the_loop_keeps_the_cap_for_the_whole_read() {
    let seed = "procedure SeedO()
         var O: Record \"QJ Order\";
         begin
            O.\"Entry No.\" := 1; O.Insert();
            O.\"Entry No.\" := 2; O.Insert();
            O.\"Entry No.\" := 3; O.Insert();
            O.\"Entry No.\" := 4; O.Insert();
         end;";
    let r = run(&with(
        seed,
        "procedure F(): Text
        var Q: Query \"QTP Top Two\"; O: Record \"QJ Order\"; Seen: Text;
        begin
            SeedO();
            Q.Open();
            while Q.Read() do begin
                Seen += Format(Q.EntryNo) + ';';
                O.Get(Q.EntryNo); O.Amount := 1; O.Modify();
            end;
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "4;3;");
}

// query/TestQueryReadAfterWrite.al:181-200 a write to a temporary record is not a write to the table.
#[test]
fn a_temporary_record_write_does_not_invalidate_the_result() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QRW Entries Unordered\"; T: Record \"QJ Order\" temporary; O: Record \"QJ Order\"; N: Integer;
        begin
            O.\"Entry No.\" := 1; O.Insert();
            O.\"Entry No.\" := 2; O.Insert();
            Q.Open();
            while Q.Read() do begin
                N += 1;
                T.\"Entry No.\" := 100 + N; T.Insert();
            end;
            Q.Close();
            exit(Format(N));
        end;",
    ));
    assert_eq!(text(r), "2");
}

// query/TestQueryRuntimeContracts.al:Query_StateMisuse_DoesNotThrow: a Read before Open is false.
#[test]
fn read_before_open_is_false() {
    let r = run(&with(
        "",
        "procedure F(): Boolean
        var Q: Query \"ALT Universal Query\";
        begin
            exit(Q.Read());
        end;",
    ));
    assert!(matches!(r, Ok(Value::Boolean(false))), "{r:?}");
}

const SEED_ROWS: &str = "
    procedure SeedRows()
    var R: Record \"QDTF Row\";
    begin
        R.Code := 'A-CLOSED'; R.Status := R.Status::Closed; R.Amount := 500; R.Kind := 1; R.Insert();
        R.Code := 'B-OPEN'; R.Status := R.Status::Open; R.Amount := 150; R.Kind := 69980; R.Insert();
        R.Code := 'C-HELD'; R.Status := R.Status::Held; R.Amount := 500; R.Kind := 0; R.Insert();
        R.Code := 'D-OPEN'; R.Status := R.Status::Open; R.Amount := 50; R.Kind := 60497; R.Insert();
    end;
";

// query/TestQueryDataItemTableFilter.al:69-135 DataItemTableFilter selects rows on a field the query
// does not project; two conditions on different fields both apply.
#[test]
fn data_item_table_filter_selects_rows_and_combines_conditions() {
    let r = run(&with(
        SEED_ROWS,
        "procedure F(): Text
        var Q: Query \"QDTF Open Rows\"; T: Query \"QDTF Two Field Filter\"; Seen: Text; Total: Decimal;
        begin
            SeedRows();
            Q.Open(); while Q.Read() do begin Seen += Q.RowCode + ';'; Total += Q.RowAmount; end; Q.Close();
            T.Open(); while T.Read() do Seen += '/' + T.RowCode; T.Close();
            exit(Seen + Format(Total));
        end;",
    ));
    assert_eq!(text(r), "B-OPEN;D-OPEN;/B-OPEN200");
}

// query/TestQueryDataItemTableFilter.al:137-179 a runtime filter on a column narrows the table filter;
// :202-251 a filter() element over the very field the table filter names combines with it.
#[test]
fn runtime_filters_combine_with_the_table_filter() {
    let r = run(&with(
        SEED_ROWS,
        "procedure F(): Text
        var Q: Query \"QDTF Open Rows\"; S: Query \"QDTF Status Filtered\"; R: Record \"QDTF Row\"; Seen: Text;
        begin
            SeedRows();
            Q.SetRange(RowCode, 'A-CLOSED');
            Q.Open(); while Q.Read() do Seen += 'a'; Q.Close();
            Q.SetRange(RowCode, 'B-OPEN');
            Q.Open(); while Q.Read() do Seen += Q.RowCode; Q.Close();
            S.SetRange(RowStatus, R.Status::Open);
            S.Open(); while S.Read() do Seen += '/' + S.RowCode; S.Close();
            S.SetRange(RowStatus, R.Status::Closed);
            S.Open(); while S.Read() do Seen += '/x'; S.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "B-OPEN/B-OPEN/D-OPEN");
}

// query/TestQueryDataItemTableFilterDatabaseConst.al:158-190: Database::"T" in a table filter is T's id.
#[test]
fn database_const_in_a_table_filter_is_the_object_id() {
    let r = run(&with(
        SEED_ROWS,
        "procedure F(): Text
        var Q: Query \"QDB Kind Filtered\"; Seen: Text;
        begin
            SeedRows();
            Q.Open(); while Q.Read() do Seen += Q.RowCode + ';'; Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "D-OPEN;");
}

// query/TestQueryFilterElementGroupBy.al:67-107 a filter() element is not a grouping column, and a
// filter set on it restricts the rows before they are aggregated.
#[test]
fn filter_element_is_a_where_condition_and_not_a_group_key() {
    let seed = "procedure SeedE()
        var H: Record \"QFG Header\"; E: Record \"QFG Entry\";
        begin
            H.\"No.\" := 'A'; H.Insert(); H.\"No.\" := 'B'; H.Insert();
            E.\"Entry No.\" := 1; E.\"Header No.\" := 'A'; E.\"Posting Date\" := 20260705D; E.Amount := 1000; E.Insert();
            E.\"Entry No.\" := 2; E.\"Header No.\" := 'A'; E.\"Posting Date\" := 20260610D; E.Amount := 100; E.Insert();
            E.\"Entry No.\" := 3; E.\"Header No.\" := 'A'; E.\"Posting Date\" := 20260620D; E.Amount := 50; E.Insert();
            E.\"Entry No.\" := 4; E.\"Header No.\" := 'B'; E.\"Posting Date\" := 20260705D; E.Amount := 9; E.Insert();
        end;";
    let r = run(&with(
        seed,
        "procedure F(): Text
        var Q: Query \"QFG Join Totals\"; Seen: Text;
        begin
            SeedE();
            Q.Open();
            while Q.Read() do Seen += StrSubstNo('%1:%2:%3;', Q.HeaderNo, Format(Q.TotalAmount, 0, 9), Q.EntryCount);
            Q.Close();
            Q.SetRange(DateFilter, 20260601D, 20260630D);
            Q.Open();
            while Q.Read() do Seen += StrSubstNo('%1:%2:%3;', Q.HeaderNo, Format(Q.TotalAmount, 0, 9), Q.EntryCount);
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "A:1150:3;B:9:1;A:150:2;");
}

// query/TestQueryDataItemLinkMultiField.al:58-90 both equalities of a DataItemLink constrain the join.
#[test]
fn every_equality_of_a_data_item_link_constrains_the_join() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QDTF Composite Link\"; H: Record \"QDTF Header\"; L: Record \"QDTF Line\"; Seen: Text;
        begin
            H.\"No.\" := 'H1'; H.\"Variant Code\" := 'BLUE'; H.Descr := 'H1-BLUE'; H.Insert();
            H.\"No.\" := 'H1'; H.\"Variant Code\" := 'RED'; H.Descr := 'H1-RED'; H.Insert();
            L.\"Header No.\" := 'H1'; L.\"Variant Code\" := 'RED'; L.\"Line No.\" := 10000; L.Tag := 'RED-LINE'; L.Insert();
            Q.Open(); while Q.Read() do Seen += Q.HdrDescr + '/' + Q.LineTag + ';'; Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "H1-RED/RED-LINE;");
}

// query/TestQueryFlowFieldColumn.al:125-175 a FlowField column is calculated per row; an unset flow
// filter constrains nothing and one set through a filter() element narrows the aggregate.
#[test]
fn flow_field_column_is_calculated_with_the_flow_filter_of_the_query() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QFF Header Dated\"; H: Record \"QFF Header\"; L: Record \"QFF Line\"; Seen: Text;
        begin
            H.\"No.\" := 'H5'; H.Insert();
            L.\"Entry No.\" := 1; L.\"Header No.\" := 'H5'; L.\"Posting Date\" := 20240110D; L.Amount := 10; L.Insert();
            L.\"Entry No.\" := 2; L.\"Header No.\" := 'H5'; L.\"Posting Date\" := 20240210D; L.Amount := 4; L.Insert();
            L.\"Entry No.\" := 3; L.\"Header No.\" := 'H5'; L.\"Posting Date\" := 20240310D; L.Amount := 1; L.Insert();
            Q.SetRange(No, 'H5');
            Q.Open(); while Q.Read() do Seen += Format(Q.DatedAmount) + ';'; Q.Close();
            Q.SetFilter(DateFilter, '%1..%2', 20240101D, 20240215D);
            Q.Open(); while Q.Read() do Seen += Format(Q.DatedAmount) + ';'; Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "15;14;");
}

// Base Application's query "Cust. Ledg. Entry Remain. Amt." sums the FlowField "Remaining Amt. (LCY)" (426 Tests-ERM tests):
// Method = Sum over a FlowField adds the value calculated for each row. Not pinned by the corpus (docs/probes-needed.md).
#[test]
fn sum_over_a_flow_field_adds_the_value_calculated_per_row() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QFF Header Sum\"; H: Record \"QFF Header\"; L: Record \"QFF Line\"; Seen: Text;
        begin
            H.\"No.\" := 'H1'; H.Insert(); H.\"No.\" := 'H2'; H.Insert();
            L.\"Entry No.\" := 1; L.\"Header No.\" := 'H1'; L.Amount := 10; L.Insert();
            L.\"Entry No.\" := 2; L.\"Header No.\" := 'H1'; L.Amount := 4; L.Insert();
            L.\"Entry No.\" := 3; L.\"Header No.\" := 'H2'; L.Amount := 1; L.Insert();
            Q.Open(); while Q.Read() do Seen += Format(Q.Total) + ';'; Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "15;");
}

// query/TestQueryOuterJoinDescendingSaveAsContracts.al: rows that tie under a descending OrderBy
// are read (their order, ascending by primary key, is pinned by the corpus test).
#[test]
fn ties_under_a_descending_order_by_are_read() {
    let r = run(&with(
        "",
        "procedure F(): Boolean
        var Q: Query \"QRO Descending Amount\"; O: Record \"QJ Order\";
        begin
            O.\"Entry No.\" := 1; O.Amount := 10; O.Insert();
            O.\"Entry No.\" := 2; O.Amount := 10; O.Insert();
            Q.Open();
            exit(Q.Read());
        end;",
    ));
    assert!(matches!(r, Ok(Value::Boolean(true))), "{r:?}");
}

// ---- mutation-testing additions (docs/status.md debt) ----

const SEED_NUM: &str = "
    procedure SeedN()
    var N: Record \"QX Num\"; Big: BigInteger;
    begin
        Big := 2147483647; Big := Big * 2;
        N.\"No.\" := 1; N.Big := Big; N.Dec := 7.5; N.Grp := 'A'; N.Insert();
        N.\"No.\" := 2; N.Big := -3; N.Dec := 2.5; N.Grp := 'A'; N.Insert();
        N.\"No.\" := 3; N.Big := 4; N.Dec := 9; N.Grp := 'A'; N.Insert();
    end;
";

// query/TestQueryReverseSign.al:36-77 pins the sign flip for Decimal; Integer and BigInteger
// columns flip the same way (they share the code path, and a BigInteger past Int32 keeps its size).
#[test]
fn reverse_sign_negates_integer_and_big_integer_columns_too() {
    let r = run(&with(
        SEED_NUM,
        "procedure F(): Text
        var Q: Query \"QX Reverse Sign Types\"; Seen: Text;
        begin
            SeedN();
            Q.Open();
            while Q.Read() do Seen += StrSubstNo('%1/%2/%3/%4;', Q.No, Q.NoReversed, Q.BigReversed, Q.DecReversed);
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "1/-1/-4294967294/-7.5;2/-2/3/-2.5;3/-3/-4/-9;");
}

// query/TestQueryReverseSign.al:79-125: the group total is negated.
#[test]
fn reverse_sign_negates_a_group_total() {
    let r = run(&with(
        SEED_NUM,
        "procedure F(): Text
        var Q: Query \"QX Reverse Sign Total\"; Seen: Text;
        begin
            SeedN();
            Q.Open();
            while Q.Read() do Seen += Q.Grp + Format(Q.DecTotal);
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "A-19");
}

// query/TestQueryAggregation.al:36-80: Min and Max pick the smallest and the largest value
// wherever it sits in the group (the corpus rows come in ascending order, so the first row is
// already the Min).
#[test]
fn min_and_max_find_the_extremes_wherever_they_sit() {
    let r = run(&with(
        SEED_NUM,
        "procedure F(): Text
        var Q: Query \"QX Min Max\"; Seen: Text;
        begin
            SeedN();
            Q.Open();
            while Q.Read() do Seen += Q.Grp + ':' + Format(Q.Lowest) + '/' + Format(Q.Highest);
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "A:2.5/9");
}

// GetFilter of a column that has a static ColumnFilter and no runtime filter is not pinned
// (docs/probes-needed.md), while a column without any filter reads empty.
#[test]
fn get_filter_of_a_statically_filtered_column_is_refused_and_of_a_plain_one_is_empty() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QJ Order Filtered\";
        begin
            exit('[' + Q.GetFilter(Amount) + ']');
        end;",
    ));
    assert_eq!(text(r), "[]");
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QJ Order Filtered\";
        begin
            exit(Q.GetFilter(CustNo));
        end;",
    ));
    assert!(unsupported(r).contains("static ColumnFilter"));
}

// query/TestQueryReadAfterWrite.al:102-179 for a join: a write to the *second* table the query
// reads also invalidates the result set, so a child row inserted after the position is read.
#[test]
fn a_write_to_the_child_table_of_a_join_is_noticed_between_reads() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QJ Cust Orders Inner\"; O: Record \"QJ Order\"; Seen: Text;
        begin
            SeedJ();
            Q.SetRange(CustNo, 'C1');
            Q.Open();
            while Q.Read() do begin
                Seen += Format(Q.EntryNo) + ',';
                if Q.EntryNo = 1 then begin
                    O.\"Entry No.\" := 9; O.\"Customer No.\" := 'C1'; O.Amount := 1; O.Insert();
                end;
            end;
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "1,2,9,");
}

// query/TestQueryDataItemTableFilterDatabaseConst.al:96-107 (a query's own procedure sets its
// filter without raising) and :109-136 (the filter applies when the query is opened).
#[test]
fn a_query_procedure_sets_the_queries_own_filters_and_the_query_reads_them() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QC With Code\"; R: Record \"QC Row\"; Seen: Text;
        begin
            R.Id := 1; R.Kind := 1; R.Amt := 5; R.Insert();
            R.Id := 2; R.Kind := 2; R.Amt := 7; R.Insert();
            R.Id := 3; R.Kind := 2; R.Amt := 9; R.Insert();
            R.Reset();
            Q.SetKind(2, R);
            Q.Open();
            while Q.Read() do Seen += Format(Q.Total) + ',';
            Q.Close();
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "16,");
}

// handlers/TestManualObjectIntegrationEvent.al:69-89: a query's procedure runs and raises an
// IntegrationEvent it declares; the subscriber fires.
#[test]
fn a_query_procedure_raises_the_integration_event_the_query_declares() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QC With Code\"; L: Record \"QC Log\"; Seen: Text;
        begin
            Q.Raise(13);
            if L.FindSet() then
                repeat Seen += L.Name + Format(L.Val) + ','; until L.Next() = 0;
            exit(Seen);
        end;",
    ));
    assert_eq!(text(r), "fired13,ran13,");
}

// query/TestQueryObject.al reads queries without code; with an OnBeforeOpen whose body is empty
// (Base App's query 1302 has one) Open and Read behave the same.
#[test]
fn an_empty_on_before_open_changes_nothing() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QC With Code\"; R: Record \"QC Row\";
        begin
            R.Id := 1; R.Kind := 1; R.Amt := 5; R.Insert();
            Q.Open();
            exit(Format(Q.Read()) + Format(Q.Total));
        end;",
    ));
    assert_eq!(text(r), "Yes5");
}

// query/TestQueryOnBeforeOpenVariablesContracts.al:104-140: OnBeforeOpen runs inside Open after the
// filters set so far, may change them, and the query's variables live as long as the query variable
// (Close and Open keep them); Clear starts again.
#[test]
fn on_before_open_sees_the_filters_and_the_variables_live_with_the_query() {
    let r = run(&with(
        "",
        "procedure Rows(var Q: Query \"QC With Variable\"): Text
        var s: Text;
        begin
            Q.Open();
            while Q.Read() do s += Format(Q.Id) + ' ';
            exit(s);
        end;
        procedure F(): Text
        var Q: Query \"QC With Variable\"; R: Record \"QC Row\"; L: Record \"QC Log\"; s: Text;
        begin
            R.Id := 1; R.Insert(); R.Id := 2; R.Insert(); R.Id := 3; R.Insert();
            Q.SetFilter(Id, '>=2');
            s := Rows(Q) + '/' + Format(Q.Opens()) + ';';
            Q.Close();
            s += Rows(Q) + ';';
            Q.Close();
            s += Q.GetFilters() + ';';
            s += Rows(Q) + '/' + Format(Q.Opens()) + ';';
            Q.Close();
            Clear(Q);
            Q.Open();
            s += Format(Q.Opens());
            if L.FindSet() then repeat s += '|' + L.Name; until L.Next() = 0;
            exit(s);
        end;",
    ));
    assert_eq!(
        text(r),
        "2 3 /1;3 ;Id: 3;3 /3;1|0open1[Id: >=2]|1open2[Id: >=2]|2open3[Id: 3]|3open1[]"
    );
}

// Base App's query 1302 groups with no OrderBy. Several groups come sorted by their grouping columns
// (corpus query/PWQueryContracts.al:165-178); one group (or none) has no order to get wrong
// (record/TestSalesQuoteVatRounding.al:67-79 reads a single VAT % group).
#[test]
fn groups_without_an_order_by_come_sorted_by_their_grouping_columns() {
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QC Unordered Groups\"; R: Record \"QC Row\";
        begin
            R.Id := 1; R.Kind := 1; R.Amt := 5; R.Insert();
            R.Id := 2; R.Kind := 1; R.Amt := 7; R.Insert();
            Q.Open();
            exit(Format(Q.Read()) + Format(Q.Total) + Format(Q.Read()));
        end;",
    ));
    assert_eq!(text(r), "Yes12No");
    let r = run(&with(
        "",
        "procedure F(): Text
        var Q: Query \"QC Unordered Groups\"; R: Record \"QC Row\";
        begin
            R.Id := 1; R.Kind := 1; R.Amt := 5; R.Insert();
            R.Id := 2; R.Kind := 2; R.Amt := 7; R.Insert();
            Q.Open();
            exit(Format(Q.Read()));
        end;",
    ));
    assert_eq!(text(r), "Yes");
}

// query/TestQueryCrossJoinContracts.al:162-226: SqlJoinType = CrossJoin without a DataItemLink gives the
// product (outer key order, then inner), the table filter of each item applies, a Sum of the inner
// item is the same for every outer row, and an empty inner table gives no rows.
#[test]
fn cross_join_gives_the_product_of_both_tables() {
    let seed =
        "R.Id := 1; R.Amt := 1; R.Insert(); R.Id := 2; R.Amt := 2; R.Insert(); R.Id := 3; R.Amt := 3; R.Insert();
                L.Name := 'X'; L.Insert(); L.Name := 'Y'; L.Insert();";
    let r = run(&with(
        "",
        &format!(
            "procedure F(): Text
        var Q: Query \"QC Cross\"; SQ: Query \"QC Cross Sum\"; R: Record \"QC Row\"; L: Record \"QC Log\"; s: Text;
        begin
            {seed}
            Q.Open(); while Q.Read() do s += Format(Q.Id) + Q.Name + ' ';
            SQ.Open(); s += '|'; while SQ.Read() do s += Format(SQ.Id) + '=' + Format(SQ.SumAmt) + ' ';
            L.DeleteAll();
            Q.Close(); Q.Open(); s += '|'; while Q.Read() do s += Format(Q.Id) + Q.Name + ' ';
            exit(s);
        end;"
        ),
    ));
    assert_eq!(text(r), "2X 2Y 3X 3Y |1=6 2=6 3=6 |");
}

#[test]
fn cross_join_with_a_data_item_link_is_refused() {
    let r = run(&with(
        "",
        "procedure F(): Text var Q: Query \"QC Cross Linked\"; begin exit(''); end;",
    ));
    assert!(unsupported(r).contains("CrossJoin"), "refusal");
}

// Groups that tie on every OrderBy column come in the order of their grouping columns, on an assumption
// (grouped-query-ties-in-grouping-column-order); nothing in the corpus pins the order of such rows.
#[test]
fn grouped_rows_that_tie_on_the_order_by_come_in_grouping_column_order() {
    let r = run("procedure F(): Text
        var Q: Query \"QX Tie Groups\"; N: Record \"QX Num\"; Seen: Text;
        begin
            N.\"No.\" := 1; N.Grp := 'A'; N.Big := 30; N.Dec := 1; N.Insert();
            N.\"No.\" := 2; N.Grp := 'A'; N.Big := 10; N.Dec := 2; N.Insert();
            N.\"No.\" := 3; N.Grp := 'A'; N.Big := 20; N.Dec := 4; N.Insert();
            Q.Open();
            while Q.Read() do
                Seen += StrSubstNo('%1:%2:%3;', Q.Grp, Q.Big, Q.DecTotal);
            exit(Seen);
        end;");
    assert_eq!(text(r), "A:10:2;A:20:4;A:30:1;");
}

/// The objects of misc/lt-probes-needed-1/PnAQueryObjects.al:8-43.
const PNA_PLAIN: &str = r#"
table 70123 "PnA Src"
{
    fields
    {
        field(1; Entry; Integer) { }
        field(2; Grp; Code[10]) { }
        field(3; Name; Text[30]) { }
        field(4; Qty; Integer) { }
        field(5; Amt; Decimal) { }
    }
    keys { key(PK; Entry) { Clustered = true; } }
}
query 70124 "PnA Plain"
{
    QueryType = Normal;
    OrderBy = ascending(Qty);
    elements
    {
        dataitem(S; "PnA Src")
        {
            column(Entry; Entry) { }
            column(Txt; Name) { }
            column(Qty; Qty) { }
            column(Amt; Amt) { }
            filter(Grp; Grp) { }
        }
    }
}
"#;

fn run_pna(body: &str) -> Result<Value, Signal> {
    let src = format!("{PNA_PLAIN}\ncodeunit 50100 C {{ {body} }}");
    al_testkit::run_in(&src, "C", "F")
}

// GetFilters with a filter element: misc/lt-probes-needed-1/PnAQueryFiltersColumnNo.al:59-118 (the
// filter element lands by its field number among the filters set before it; a column lands by
// column order among the columns only) and query/TestQueryRuntimeContracts.al:219-268.
#[test]
fn get_filters_with_a_filter_element_follows_the_order_of_setting() {
    let r = run_pna(
        "procedure F(): Text
        var Q: Query \"PnA Plain\"; s: Text;
        begin
            Q.SetRange(Grp, 'G'); Q.SetRange(Entry, 1); Q.SetRange(Qty, 3);
            s := Q.GetFilters() + ';';
            Clear(Q);
            Q.SetRange(Entry, 1); Q.SetRange(Qty, 3); Q.SetRange(Grp, 'G');
            s += Q.GetFilters() + ';';
            Clear(Q);
            Q.SetRange(Txt, 'a'); Q.SetRange(Grp, 'G'); Q.SetRange(Qty, 3);
            s += Q.GetFilters() + ';';
            Clear(Q);
            Q.SetRange(Qty, 3); Q.SetRange(Txt, 'a'); Q.SetRange(Grp, 'G');
            s += Q.GetFilters();
            exit(s);
        end;",
    );
    assert_eq!(
        text(r),
        "Grp: G, Entry: 1, Qty: 3;Entry: 1, Grp: G, Qty: 3;Grp: G, Name: a, Qty: 3;Grp: G, Name: a, Qty: 3"
    );
}

// Setting a filter again or removing one with a filter element present is not pinned.
#[test]
fn get_filters_with_a_filter_element_after_a_filter_was_set_again_is_refused() {
    let r = run_pna(
        "procedure F(): Text
        var Q: Query \"PnA Plain\";
        begin
            Q.SetRange(Grp, 'G'); Q.SetRange(Entry, 1); Q.SetRange(Grp, 'H');
            exit(Q.GetFilters());
        end;",
    );
    assert!(unsupported(r).contains("GetFilters"));
}

/// The objects of errors/lt-ms-refusals-133/P133QueryOuterFilters.al:8-122.
const P133: &str = r#"
table 74054 "P133 Parent"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; DocNo; Code[20]) { }
        field(2; PostDate; Date) { }
        field(3; Amount; Decimal) { }
        field(4; Debit; Decimal) { }
    }

    keys
    {
        key(PK; DocNo, PostDate) { Clustered = true; }
    }
}

table 74055 "P133 Child"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; EntryNo; Integer) { }
        field(2; DocNo; Code[20]) { }
        field(3; PostDate; Date) { }
        field(4; Kind; Integer) { }
    }

    keys
    {
        key(PK; EntryNo) { Clustered = true; }
    }
}

query 74056 "P133 Q CountZero Default"
{
    QueryType = Normal;
    OrderBy = ascending(DocNo);

    elements
    {
        dataitem(Parent; "P133 Parent")
        {
            column(DocNo; DocNo) { }
            column(PostDate; PostDate) { }
            column(SumAmount; Amount) { Method = Sum; }
            column(SumDebit; Debit) { Method = Sum; }
            column(NoOfParents) { Method = Count; }
            dataitem(Child; "P133 Child")
            {
                DataItemLink = DocNo = Parent.DocNo, PostDate = Parent.PostDate;
                column(NoOfChildren)
                {
                    ColumnFilter = NoOfChildren = const(0);
                    Method = Count;
                }
            }
        }
    }
}

query 74057 "P133 Q CountZero Left"
{
    QueryType = Normal;
    OrderBy = ascending(DocNo);

    elements
    {
        dataitem(Parent; "P133 Parent")
        {
            column(DocNo; DocNo) { }
            column(PostDate; PostDate) { }
            column(SumAmount; Amount) { Method = Sum; }
            column(NoOfParents) { Method = Count; }
            dataitem(Child; "P133 Child")
            {
                DataItemLink = DocNo = Parent.DocNo, PostDate = Parent.PostDate;
                SqlJoinType = LeftOuterJoin;
                column(NoOfChildren)
                {
                    ColumnFilter = NoOfChildren = const(0);
                    Method = Count;
                }
            }
        }
    }
}

query 74058 "P133 Q CountPositive Left"
{
    QueryType = Normal;
    OrderBy = ascending(DocNo);

    elements
    {
        dataitem(Parent; "P133 Parent")
        {
            column(DocNo; DocNo) { }
            column(PostDate; PostDate) { }
            column(NoOfParents) { Method = Count; }
            dataitem(Child; "P133 Child")
            {
                DataItemLink = DocNo = Parent.DocNo, PostDate = Parent.PostDate;
                SqlJoinType = LeftOuterJoin;
                column(NoOfChildren)
                {
                    ColumnFilter = NoOfChildren = filter(> 0);
                    Method = Count;
                }
            }
        }
    }
}
"#;

fn run_p133(body: &str) -> Result<Value, Signal> {
    let src = format!("{P133}\ncodeunit 50100 C {{ {body} }}");
    al_testkit::run_in(&src, "C", "F")
}

const P133_SEED: &str = "
    procedure P(DocNo: Code[20]; Amount: Decimal)
    var R: Record \"P133 Parent\";
    begin
        R.DocNo := DocNo; R.PostDate := DMY2Date(1, 1, 2024); R.Amount := Amount; R.Debit := Amount / 10; R.Insert();
    end;
    procedure K(EntryNo: Integer; DocNo: Code[20]; Day: Integer)
    var R: Record \"P133 Child\";
    begin
        R.EntryNo := EntryNo; R.DocNo := DocNo; R.PostDate := DMY2Date(Day, 1, 2024); R.Kind := 1; R.Insert();
    end;
    procedure Seed()
    begin
        P('D1', 10); P('D2', 20); P('D3', 30); P('D4', 40);
        K(1, 'D1', 1); K(2, 'D1', 1); K(3, 'D2', 1); K(4, 'D4', 2);
    end;
";

// A Count column on the outer-joined child with ColumnFilter = const(0) keeps the parents that have
// no child, with the default join type and with LeftOuterJoin spelled out
// (errors/lt-ms-refusals-133/P133QueryOuterFilters.al:321-323, :361-362).
#[test]
fn count_zero_column_filter_on_the_outer_child_keeps_the_parents_without_children() {
    let r = run_p133(&format!(
        "{P133_SEED}
        procedure F(): Text
        var D: Query \"P133 Q CountZero Default\"; L: Query \"P133 Q CountZero Left\"; s: Text;
        begin
            Seed();
            D.Open();
            while D.Read() do s += D.DocNo + ':' + Format(D.NoOfParents) + ',' + Format(D.NoOfChildren) + ',' + Format(D.SumAmount) + ' ';
            s += '|';
            L.Open();
            while L.Read() do s += L.DocNo + ':' + Format(L.NoOfParents) + ',' + Format(L.NoOfChildren) + ',' + Format(L.SumAmount) + ' ';
            exit(s);
        end;"
    ));
    assert_eq!(text(r), "D3:1,0,30 D4:1,0,40 |D3:1,0,30 D4:1,0,40 ");
}

const TOP_WHILE_OPEN: &str = r#"
table 60870 "QX Num"
{
    fields { field(1; "No."; Integer) { } field(4; Grp; Code[10]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
query 60890 "QX Top Open"
{
    QueryType = Normal;
    elements
    {
        dataitem(Num; "QX Num")
        {
            column(No; "No.") { }
            filter(Grp; Grp) { }
        }
    }
}
codeunit 50100 C
{
    var Q: Query "QX Top Open";
    procedure Seed()
    var N: Record "QX Num"; i: Integer;
    begin
        for i := 1 to 5 do begin N."No." := i; N.Grp := 'A'; N.Insert(); end;
    end;
    procedure ReadAll() Rows: Text
    begin
        while Q.Read() do Rows += Format(Q.No) + ',';
    end;
    [TryFunction]
    procedure TryTop(N: Integer)
    begin
        Q.TopNumberOfRows(N);
    end;
    [TryFunction]
    procedure TryReadAll(var Rows: Text)
    begin
        Rows := ReadAll();
    end;
    procedure TryRead(): Text
    var Rows: Text;
    begin
        if TryReadAll(Rows) then exit('Yes ' + Rows);
        exit('No ' + GetLastErrorCode());
    end;
    // misc/lt-probes-needed-1/PnAQueryOpenRead.al:247-290 PX_TopNumberOfRows_AroundOpen.
    procedure AroundOpen(): Text
    var Seen: Text;
    begin
        Seed();
        Q.Open();
        Seen += Format(TryTop(2)) + ' ' + TryRead() + '|';
        Q.Close();
        Clear(Q);
        Q.TopNumberOfRows(2);
        Q.Open();
        Seen += TryRead() + '|';
        Clear(Q);
        Q.Open();
        Seen += Format(Q.Read()) + ' ' + Format(TryTop(1)) + ' ' + TryRead() + '|';
        Q.Open();
        exit(Seen + TryRead());
    end;
    // Writer TH: PnAQueryOpenRead.al TH_TopAfterEmptyOpen_ThenReopen (al-runner issue 6039).
    procedure AfterEmptyOpen(): Text
    var Seen: Text;
    begin
        Seed();
        Q.SetRange(Grp, 'X');
        Q.TopNumberOfRows(1);
        Seen += Format(Q.Open()) + ' ' + Format(Q.Read()) + ' ';
        Q.SetRange(Grp);
        Seen += Format(TryTop(1)) + ' ' + Format(Q.TopNumberOfRows()) + ' ' + Format(Q.Open()) + ' ';
        exit(Seen + TryRead());
    end;
    // Writer TH: TH_TopAfterFullRead_ThenReopen and TH_TopTwiceWhileOpen_LastValueCounts.
    procedure AfterFullRead(): Text
    var Seen: Text;
    begin
        Seed();
        Q.Open();
        Seen += ReadAll() + ' ' + Format(TryTop(2)) + ' ' + Format(Q.TopNumberOfRows()) + ' ' + Format(Q.Open()) + ' ' + TryRead() + '|';
        Q.TopNumberOfRows(1);
        Q.TopNumberOfRows(3);
        Seen += TryRead() + ' ';
        Q.Open();
        exit(Seen + TryRead());
    end;
}
"#;

// misc/lt-probes-needed-1/PnAQueryOpenRead.al:247-290 PX_TopNumberOfRows_AroundOpen: TopNumberOfRows after Open (before or
// after a Read) raises nothing, the next Read raises the metadata-changed error (code Query), and Open again takes the new top.
#[test]
fn top_number_of_rows_on_an_open_query_applies_to_the_next_open() {
    al_builtins::assumptions::clear_hits();
    let r = al_testkit::run_in(TOP_WHILE_OPEN, "C", "AroundOpen");
    assert_eq!(text(r), "Yes No Query|Yes 1,2,|Yes Yes No Query|Yes 1,");
    assert_eq!(al_builtins::assumptions::take_hits(), vec![]);
}

// Writer TH (al-runner issue 6039): the same on a query read to its end (PnAQueryOpenRead.al TH_*).
#[test]
fn top_number_of_rows_on_a_query_read_to_the_end_applies_to_the_next_open() {
    al_builtins::assumptions::clear_hits();
    let r = al_testkit::run_in(TOP_WHILE_OPEN, "C", "AfterEmptyOpen");
    assert_eq!(text(r), "Yes No Yes 1 Yes Yes 1,");
    let r = al_testkit::run_in(TOP_WHILE_OPEN, "C", "AfterFullRead");
    assert_eq!(text(r), "1,2,3,4,5, Yes 2 Yes Yes 1,2,|No Query Yes 1,2,3,");
    assert_eq!(al_builtins::assumptions::take_hits(), vec![]);
}
