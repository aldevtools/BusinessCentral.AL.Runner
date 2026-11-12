//! The text of a Decimal column in the report dataset, by the shape of the column's source (al-runner issue 7440).
//! reports/dataset/DatasetDecimalPlacesValueOrigins.al pins the shapes that write a Decimal without trailing zeros: a field of
//! the data item by name (:47) or with the item's name (:52), a field of a global Record variable (:71), a global Decimal
//! variable (:79), and an expression over a stored field of the data item's row (:53 `D05 * 1`). Every other shape writes the
//! value with its scale (reports/layouts-saveas/TestReportSaveAsColumnTypesContracts.al:309 `Dec * 1.5` is `18.5185170`;
//! DatasetDecimalPlacesValueOrigins.al:425-570 for the other shapes).

use al_testkit::text;

const FIXTURE: &str = r#"
table 50970 "DC Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; D; Decimal) { DecimalPlaces = 0 : 5; }
        field(3; B; Blob) { }
        field(4; S; Decimal) { FieldClass = FlowField; CalcFormula = sum("DC Row".D); }
        field(5; D2; Decimal) { DecimalPlaces = 0 : 5; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50971 "DC Helper"
{
    procedure GetV(): Decimal
    begin
        exit(Round(1, 0.00001));
    end;
}

report 50972 "DC Dec"
{
    UsageCategory = None;
    dataset
    {
        dataitem(Row; "DC Row")
        {
            DataItemTableView = sorting(Id) where(Id = const(1));
            column(Name; D) { }
            column(Qualified; Row.D) { }
            column(Times1; D * 1) { }
            column(GlobalRecord; Mem.D) { }
            column(VarName; V) { }
            column(GetName; G.D) { }
            column(GetTimes2; G.D * 2) { }
            column(VarTimes1; V * 1) { }
            column(ArrElem; Arr[1]) { }
            column(Neg; -V) { }
            column(Paren; (V)) { }
            column(CallNoParens; GetV) { }
            column(CallParens; GetV()) { }
            column(MemberCall; Helper.GetV) { }
            column(SetInTrigger; W * 1) { }
            column(FlowTimes1; S * 1) { }
            column(FlowName; S) { }
            trigger OnAfterGetRecord()
            begin
                G.Get(2);
                V := Round(1, 0.00001);
                Arr[1] := Round(1, 0.00001);
                W := 2.50;
                CalcFields(S);
            end;
        }
    }
    trigger OnPreReport()
    begin
        Mem.D := Round(1, 0.00001);
    end;
    var
        G: Record "DC Row";
        Mem: Record "DC Row";
        Helper: Codeunit "DC Helper";
        V: Decimal;
        W: Decimal;
        Arr: array[2] of Decimal;

    procedure GetV(): Decimal
    begin
        exit(Round(1, 0.00001));
    end;
}

report 50973 "DC Temp"
{
    UsageCategory = None;
    dataset
    {
        dataitem(Tmp; "DC Row")
        {
            UseTemporary = true;
            column(TmpName; D) { }
            column(TmpTimes1; D * 1) { }
        }
    }
    trigger OnPreReport()
    begin
        Tmp.Id := 1;
        Tmp.D := Round(1, 0.00001);
        Tmp.Insert();
    end;
}

codeunit 50974 "DC Tests"
{
    local procedure Cols(var Tmp: Record "DC Row" temporary): Text
    var
        InS: InStream;
        Line: Text;
        R: Text;
    begin
        Tmp.B.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            if Line.Contains('<Column ') then
                R += Line.Trim() + '|';
        end;
        exit(R);
    end;

    procedure Dec(): Text
    var
        Row: Record "DC Row";
        Tmp: Record "DC Row" temporary;
        OutS: OutStream;
    begin
        Row.Id := 1;
        Row.D := Round(1, 0.00001);
        Row.Insert();
        Row.Id := 2;
        Row.D := 1.50;
        Row.Insert();
        Tmp.B.CreateOutStream(OutS);
        Report.SaveAs(Report::"DC Dec", '', ReportFormat::Xml, OutS);
        exit(Cols(Tmp));
    end;

    procedure Temp(): Text
    var
        Tmp: Record "DC Row" temporary;
        OutS: OutStream;
    begin
        Tmp.B.CreateOutStream(OutS);
        Report.SaveAs(Report::"DC Temp", '', ReportFormat::Xml, OutS);
        exit(Cols(Tmp));
    end;
}
"#;

fn column(lines: &str, name: &str) -> String {
    let start = format!("<Column name=\"{name}\"");
    lines
        .split('|')
        .find(|l| l.starts_with(&start))
        .unwrap_or_else(|| panic!("no column {name} in {lines}"))
        .to_string()
}

fn value(lines: &str, name: &str) -> String {
    let line = column(lines, name);
    let open = line.find('>').expect("a column line has a tag") + 1;
    let close = line.rfind("</Column>").expect("a column line ends");
    line[open..close].to_string()
}

/// DatasetDecimalPlacesValueOrigins.al:241-247 (`D05`, `D05Qualified`, `D05Times1` over the stored `Round(1, 0.00001)`), :364
/// (`RecRounded.D05` of a global record) and :397 (`VarRounded`, a global variable): each is `1`.
#[test]
fn the_pinned_source_shapes_write_no_trailing_zeros() {
    let lines = text(al_testkit::run_in(FIXTURE, "DC Tests", "Dec"));
    for name in ["Name", "Qualified", "Times1", "GlobalRecord", "VarName"] {
        assert_eq!(value(&lines, name), "1", "{name}: {lines}");
    }
    // `Global.Field` of a global Record variable read by Get: the shape of :364, the value 1.50 stored.
    assert_eq!(value(&lines, "GetName"), "1.5", "{lines}");
}

/// The other source shapes write what the value holds (DatasetDecimalPlacesValueOrigins.al:425-550): `Round(1, 0.00001)` is `1`
/// (al-builtins `mathfns.rs`), so a variable, an array element, unary minus, parentheses and the calls give `1`; a row read from
/// the database has no trailing zeros (al-data `MemStore::as_database`), so the stored 1.50 read by Get times two is `3.0` and the
/// FlowField over the rows 1 and 1.50 is `2.5`. A variable set from the literal 2.50 keeps its scale (`W * 1`).
#[test]
fn the_other_source_shapes_write_what_the_value_holds() {
    let lines = text(al_testkit::run_in(FIXTURE, "DC Tests", "Dec"));
    for (name, want) in [
        ("GetTimes2", "3.0"),
        ("VarTimes1", "1"),
        ("ArrElem", "1"),
        ("Neg", "-1"),
        ("Paren", "1"),
        ("CallNoParens", "1"),
        ("CallParens", "1"),
        ("MemberCall", "1"),
        ("SetInTrigger", "2.50"),
        ("FlowTimes1", "2.5"),
        ("FlowName", "2.5"),
    ] {
        assert_eq!(value(&lines, name), want, "{name}: {lines}");
    }
}

/// A temporary data item keeps its rows in memory: its field set to `Round(1, 0.00001)` is `1`, by name and times one
/// (DatasetDecimalPlacesValueOrigins.al:555-570).
#[test]
fn a_temporary_data_item_writes_what_the_value_holds() {
    let lines = text(al_testkit::run_in(FIXTURE, "DC Tests", "Temp"));
    assert_eq!(value(&lines, "TmpName"), "1", "{lines}");
    assert_eq!(value(&lines, "TmpTimes1"), "1", "{lines}");
}
