//! A table element without LinkTable right below another table element, with a SourceTableView (sorting and filter), passed once per
//! row above: every pass starts from the view alone, whatever OnPreXmlItem of an earlier pass set (filter, key, direction, Reset).
//! Corpus branch al-runtime-rs-staging-r6924x, xmlports/shapes-properties/R6924xNestedView.al (corpus PR 1144, measured on BC 29.0),
//! asked by al-runner issue 6924 (Microsoft's PEPPOL 3.0 line-level AllowanceCharge element).

use al_testkit::{run, text};

const SRC: &str = r#"
table 69960 "NV Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69961 "NV Head" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
table 69962 "NV Part" { fields { field(1; Entry; Integer) { } field(2; Kind; Code[10]) { } field(3; Qty; Integer) { } } keys { key(PK; Entry) { } key(ByKind; Kind) { } } }
xmlport 69963 "NV Port"
{
    Direction = Both; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Head; "NV Head")
            {
                fieldattribute(No; Head."No.") { }
                tableelement(Part; "NV Part")
                {
                    MinOccurs = Zero;
                    SourceTableView = sorting(Kind) where(Qty = filter(> 0));
                    fieldattribute(Entry; Part.Entry) { }
                    fieldattribute(Kind; Part.Kind) { }
                    fieldattribute(Qty; Part.Qty) { }
                    trigger OnPreXmlItem()
                    begin
                        Log += 'pre h' + Format(Head."No.") + ' f=' + Part.GetFilters() + ' n=' + Format(Part.Count()) + ' k=' + Part.CurrentKey() + ' a=' + Format(Part.Ascending()) + ';';
                        case Mode of
                            'each': case Head."No." of 1: Part.SetRange(Kind, 'A'); 2: Part.SetRange(Kind, 'B'); end;
                            'first': if Head."No." = 1 then Part.SetRange(Kind, 'A');
                            'key': if Head."No." = 1 then Part.SetCurrentKey(Entry);
                            'desc': if Head."No." = 1 then Part.Ascending(false);
                            'reset': if Head."No." = 1 then Part.Reset();
                        end;
                    end;
                    trigger OnAfterGetRecord() begin Log += 'get ' + Format(Part.Entry) + ';'; end;
                    trigger OnBeforeInsertRecord() begin Log += 'ins p' + Format(Part.Entry) + ' f=' + Part.GetFilters() + ' k=' + Part.CurrentKey() + ';'; end;
                }
                trigger OnBeforeInsertRecord() begin Log += 'ins h' + Format(Head."No.") + ';'; end;
            }
        }
    }
    var Log: Text; Mode: Text;
    procedure SetMode(M: Text) begin Mode := M; end;
    procedure GetLog(): Text begin exit(Log); end;
}
table 69966 "NV Num" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { } } }
xmlport 69964 "NV Int"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Head; "NV Head")
            {
                fieldattribute(No; Head."No.") { }
                tableelement(ChargeLoop; "NV Num")
                {
                    MinOccurs = Zero; XmlName = 'Charge';
                    SourceTableView = sorting(Number) where(Number = filter(1 ..));
                    fieldattribute(N; ChargeLoop.Number) { }
                    trigger OnPreXmlItem()
                    begin
                        Log += 'pre h' + Format(Head."No.") + ' f=' + ChargeLoop.GetFilters() + ';';
                        if Grow then ChargeLoop.SetRange(Number, 1, Head."No.") else ChargeLoop.SetRange(Number, 1, 1);
                    end;
                    trigger OnAfterGetRecord()
                    begin
                        Log += 'get h' + Format(Head."No.") + ' n' + Format(ChargeLoop.Number) + ';';
                        if Head."No." = 2 then currXMLport.Skip();
                    end;
                }
            }
        }
    }
    var Log: Text; Grow: Boolean;
    procedure SetGrow(G: Boolean) begin Grow := G; end;
    procedure GetLog(): Text begin exit(Log); end;
}
codeunit 69965 C
{
    local procedure Seed()
    var H: Record "NV Head"; N: Record "NV Num"; No: Integer;
    begin
        for No := 1 to 3 do begin H."No." := No; H.Insert(); end;
        for No := 0 to 5 do begin N.Number := No; N.Insert(); end;
        AddPart(1, 'B', 1); AddPart(2, 'A', 2); AddPart(3, 'A', 0); AddPart(4, 'C', 5); AddPart(5, 'B', 3);
    end;

    local procedure AddPart(Entry: Integer; Kind: Code[10]; Qty: Integer)
    var P: Record "NV Part";
    begin
        P.Entry := Entry; P.Kind := Kind; P.Qty := Qty; P.Insert();
    end;

    local procedure ReadAll(var B: Record "NV Blob" temporary): Text
    var InS: InStream; Line: Text; T: Text;
    begin
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line.Trim() + '|'; end;
        exit(T);
    end;

    procedure Nested(Mode: Text): Text
    var B: Record "NV Blob" temporary; X: XmlPort "NV Port"; OutS: OutStream;
    begin
        Seed();
        B.Data.CreateOutStream(OutS);
        X.SetMode(Mode);
        X.SetDestination(OutS);
        X.Export();
        exit(ReadAll(B) + '#' + X.GetLog());
    end;

    procedure NestedNone(): Text begin exit(Nested('')); end;
    procedure NestedEach(): Text begin exit(Nested('each')); end;
    procedure NestedFirst(): Text begin exit(Nested('first')); end;
    procedure NestedKey(): Text begin exit(Nested('key')); end;
    procedure NestedDesc(): Text begin exit(Nested('desc')); end;
    procedure NestedReset(): Text begin exit(Nested('reset')); end;

    procedure Int(Grow: Boolean): Text
    var B: Record "NV Blob" temporary; X: XmlPort "NV Int"; OutS: OutStream;
    begin
        Seed();
        B.Data.CreateOutStream(OutS);
        X.SetGrow(Grow);
        X.SetDestination(OutS);
        X.Export();
        exit(ReadAll(B) + '#' + X.GetLog());
    end;

    procedure IntOne(): Text begin exit(Int(false)); end;
    procedure IntGrow(): Text begin exit(Int(true)); end;

    procedure Import(): Text
    var B: Record "NV Blob" temporary; X: XmlPort "NV Port"; H: Record "NV Head"; P: Record "NV Part"; OutS: OutStream; InS: InStream; Rows: Text;
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<Root><Head No="1"><Part Entry="1" Kind="B" Qty="1" /><Part Entry="3" Kind="A" Qty="0" /></Head><Head No="2"><Part Entry="2" Kind="A" Qty="2" /><Part Entry="4" Kind="C" Qty="-1" /></Head></Root>');
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        X.SetSource(InS);
        X.Import();
        if H.FindSet() then repeat Rows += 'H' + Format(H."No.") + ';'; until H.Next() = 0;
        if P.FindSet() then repeat Rows += 'P' + Format(P.Entry) + ' ' + P.Kind + ' ' + Format(P.Qty) + ';'; until P.Next() = 0;
        exit(Rows + '#' + X.GetLog());
    end;
}
"#;

const HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|<Root>|";
const VIEW_ROWS: &str = "<Part Entry=\"2\" Kind=\"A\" Qty=\"2\" />|<Part Entry=\"1\" Kind=\"B\" Qty=\"1\" />|<Part Entry=\"5\" Kind=\"B\" Qty=\"3\" />|<Part Entry=\"4\" Kind=\"C\" Qty=\"5\" />|";
const PRE: &str = " f= n=4 k=Kind a=Yes;";

fn heads(first: &str, second: &str, third: &str) -> String {
    format!(
        "{HEAD}<Head No=\"1\">|{first}</Head>|<Head No=\"2\">|{second}</Head>|<Head No=\"3\">|{third}</Head>|</Root>|"
    )
}

fn view_log(first: &str, second: &str, third: &str) -> String {
    format!("pre h1{PRE}{first}pre h2{PRE}{second}pre h3{PRE}{third}")
}

const VIEW_GETS: &str = "get 2;get 1;get 5;get 4;";

// R6924xNestedView.al NestedView_Export_NoCode.
#[test]
fn every_pass_reads_the_view() {
    let out = text(run(SRC, "NestedNone"));
    assert_eq!(
        out,
        format!(
            "{}#{}",
            heads(VIEW_ROWS, VIEW_ROWS, VIEW_ROWS),
            view_log(VIEW_GETS, VIEW_GETS, VIEW_GETS)
        )
    );
}

// R6924xNestedView.al NestedView_Export_FilterEachPass.
#[test]
fn a_filter_of_onprexmlitem_holds_for_its_pass_only() {
    let out = text(run(SRC, "NestedEach"));
    let a = "<Part Entry=\"2\" Kind=\"A\" Qty=\"2\" />|";
    let b = "<Part Entry=\"1\" Kind=\"B\" Qty=\"1\" />|<Part Entry=\"5\" Kind=\"B\" Qty=\"3\" />|";
    assert_eq!(
        out,
        format!(
            "{}#{}",
            heads(a, b, VIEW_ROWS),
            view_log("get 2;", "get 1;get 5;", VIEW_GETS)
        )
    );
    let out = text(run(SRC, "NestedFirst"));
    assert_eq!(
        out,
        format!(
            "{}#{}",
            heads(a, VIEW_ROWS, VIEW_ROWS),
            view_log("get 2;", VIEW_GETS, VIEW_GETS)
        )
    );
}

// R6924xNestedView.al NestedView_Export_KeyFirstPass, _DescendingFirstPass, _ResetFirstPass.
#[test]
fn a_key_direction_or_reset_of_the_first_pass_does_not_reach_the_next() {
    let by_entry = "<Part Entry=\"1\" Kind=\"B\" Qty=\"1\" />|<Part Entry=\"2\" Kind=\"A\" Qty=\"2\" />|<Part Entry=\"4\" Kind=\"C\" Qty=\"5\" />|<Part Entry=\"5\" Kind=\"B\" Qty=\"3\" />|";
    let out = text(run(SRC, "NestedKey"));
    assert_eq!(
        out,
        format!(
            "{}#{}",
            heads(by_entry, VIEW_ROWS, VIEW_ROWS),
            view_log("get 1;get 2;get 4;get 5;", VIEW_GETS, VIEW_GETS)
        )
    );
    let desc = "<Part Entry=\"4\" Kind=\"C\" Qty=\"5\" />|<Part Entry=\"5\" Kind=\"B\" Qty=\"3\" />|<Part Entry=\"1\" Kind=\"B\" Qty=\"1\" />|<Part Entry=\"2\" Kind=\"A\" Qty=\"2\" />|";
    let out = text(run(SRC, "NestedDesc"));
    assert_eq!(
        out,
        format!(
            "{}#{}",
            heads(desc, VIEW_ROWS, VIEW_ROWS),
            view_log("get 4;get 5;get 1;get 2;", VIEW_GETS, VIEW_GETS)
        )
    );
    let all = "<Part Entry=\"1\" Kind=\"B\" Qty=\"1\" />|<Part Entry=\"2\" Kind=\"A\" Qty=\"2\" />|<Part Entry=\"3\" Kind=\"A\" Qty=\"0\" />|<Part Entry=\"4\" Kind=\"C\" Qty=\"5\" />|<Part Entry=\"5\" Kind=\"B\" Qty=\"3\" />|";
    let out = text(run(SRC, "NestedReset"));
    assert_eq!(
        out,
        format!(
            "{}#{}",
            heads(all, VIEW_ROWS, VIEW_ROWS),
            view_log("get 1;get 2;get 3;get 4;get 5;", VIEW_GETS, VIEW_GETS)
        )
    );
}

// R6924xNestedView.al IntegerView_Export_OneEachPass, _GrowEachPass (PEPPOL's shape; a table of numbers stands in for Integer).
#[test]
fn an_integer_element_with_setrange_at_every_pass() {
    let out = text(run(SRC, "IntOne"));
    assert_eq!(
        out,
        format!(
            "{HEAD}<Head No=\"1\">|<Charge N=\"1\" />|</Head>|<Head No=\"2\" />|<Head No=\"3\">|<Charge N=\"1\" />|</Head>|</Root>|#pre h1 f=;get h1 n1;pre h2 f=;get h2 n1;pre h3 f=;get h3 n1;"
        )
    );
    let out = text(run(SRC, "IntGrow"));
    assert_eq!(
        out,
        format!(
            "{HEAD}<Head No=\"1\">|<Charge N=\"1\" />|</Head>|<Head No=\"2\" />|<Head No=\"3\">|<Charge N=\"1\" />|<Charge N=\"2\" />|<Charge N=\"3\" />|</Head>|</Root>|#pre h1 f=;get h1 n1;pre h2 f=;get h2 n1;get h2 n2;pre h3 f=;get h3 n1;get h3 n2;get h3 n3;"
        )
    );
}

// R6924xNestedView.al NestedView_Import_RowsOutsideTheView.
#[test]
fn an_import_stores_rows_outside_the_view() {
    let out = text(run(SRC, "Import"));
    assert_eq!(
        out,
        "H1;H2;P1 B 1;P2 A 2;P3 A 0;P4 C -1;#ins p1 f= k=Kind;ins p3 f= k=Entry;ins h1;ins p2 f= k=Entry;ins p4 f= k=Entry;ins h2;"
    );
}
