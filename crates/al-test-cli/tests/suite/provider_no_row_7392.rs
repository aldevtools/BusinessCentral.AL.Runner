//! A Provider chain whose Provider part shows no row (al-runner issue 7392, the reporter's smaller shape): a Card with a
//! ListPart `Lines`, a page extension that adds a ListPart `Details` outside FactBoxes with `Provider = Lines`, on a header
//! with no lines. The cells of corpus pull request 1384 (testpage/parts/ProviderNoRowFirst.al) run as flagged passes
//! (`provider-no-row-ext-chain`); one fact changed keeps main's refusal.

use al_builtins::assumptions::Assumption::ProviderNoRowExtChain;
use al_testrun::Outcome;

use super::reporter_repros_7820_7821::outcome_and_marks;

const SRC: &str = r#"
table 69811 "PN Header"
{
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 69812 "PN Line"
{
    fields { field(1; "Header No."; Code[20]) { } field(2; "Line No."; Integer) { } }
    keys { key(PK; "Header No.", "Line No.") { Clustered = true; } }
}
table 69813 "PN Detail"
{
    fields { field(1; "Header No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Seq; Integer) { } }
    keys { key(PK; "Header No.", "Line No.", Seq) { Clustered = true; } }
}
page 69811 "PN Card"
{
    PageType = Card;
    SourceTable = "PN Header";
    ApplicationArea = All;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "PN Line Part")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No.");
            }
        }
    }
}
page 69812 "PN Line Part"
{
    PageType = ListPart;
    SourceTable = "PN Line";
    ApplicationArea = All;
    Editable = false;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { ApplicationArea = All; } } } }
}
page 69813 "PN Detail Part"
{
    PageType = ListPart;
    SourceTable = "PN Detail";
    ApplicationArea = All;
    Editable = false;
    layout { area(Content) { repeater(Rows) { field(Seq; Rec.Seq) { ApplicationArea = All; } } } }
}
pageextension 69815 "PN Card Ext" extends "PN Card"
{
    layout
    {
        addafter(Lines)
        {
            part(Details; "PN Detail Part")
            {
                ApplicationArea = All;
                Provider = Lines;
                SubPageLink = "Header No." = field("Header No."), "Line No." = field("Line No.");
            }
        }
    }
}
codeunit 69816 "PN Tests"
{
    Subtype = Test;

    local procedure Setup()
    var
        Header: Record "PN Header";
        Line: Record "PN Line";
        Detail: Record "PN Detail";
    begin
        Header.DeleteAll();
        Line.DeleteAll();
        Detail.DeleteAll();
        Header."No." := 'H1';
        Header.Insert();
        Header."No." := 'H2';
        Header.Insert();
        Line."Header No." := 'H1';
        Line."Line No." := 10000;
        Line.Insert();
        Detail."Header No." := 'H1';
        Detail."Line No." := 10000;
        Detail.Seq := 1;
        Detail.Insert();
    end;

    local procedure Check(What: Text; Have: Text; Want: Text)
    begin
        if Have <> Want then
            Error('%1: %2, expected %3', What, Have, Want);
    end;

    [Test]
    procedure NoLinesFirst()
    var
        Header: Record "PN Header";
        Card: TestPage "PN Card";
    begin
        Setup();
        Header.Get('H2');
        Card.OpenView();
        Card.GoToRecord(Header);
        Check('lines first', Format(Card.Lines.First()), 'No');
        Check('details first', Format(Card.Details.First()), 'No');
        Check('lines value', Card.Lines."Line No.".Value(), '');
        Check('details value', Card.Details.Seq.Value(), '');
        Card.Close();
    end;

    [Test]
    procedure WithLinesFirst()
    var
        Header: Record "PN Header";
        Card: TestPage "PN Card";
    begin
        Setup();
        Header.Get('H1');
        Card.OpenView();
        Card.GoToRecord(Header);
        Check('details value before', Card.Details.Seq.Value(), '1');
        Check('lines first', Format(Card.Lines.First()), 'Yes');
        Check('details first', Format(Card.Details.First()), 'Yes');
        Check('lines value', Card.Lines."Line No.".Value(), '10000');
        Check('details value', Card.Details.Seq.Value(), '1');
        Card.Close();
    end;

    [Test]
    procedure NoLinesDetailsFirstOnly()
    var
        Header: Record "PN Header";
        Card: TestPage "PN Card";
    begin
        Setup();
        Header.Get('H2');
        Card.OpenView();
        Card.GoToRecord(Header);
        Check('details first', Format(Card.Details.First()), 'No');
        Check('details value', Card.Details.Seq.Value(), '');
        Card.Close();
    end;

    [Test]
    procedure NoLinesNoMove()
    var
        Header: Record "PN Header";
        Card: TestPage "PN Card";
    begin
        Setup();
        Header.Get('H2');
        Card.OpenView();
        Card.GoToRecord(Header);
        Check('lines value', Card.Lines."Line No.".Value(), '');
        Check('details value', Card.Details.Seq.Value(), '');
        Card.Close();
    end;

    [Test]
    procedure NoLinesLastNext()
    var
        Header: Record "PN Header";
        Card: TestPage "PN Card";
    begin
        Setup();
        Header.Get('H2');
        Card.OpenView();
        Card.GoToRecord(Header);
        Check('lines last', Format(Card.Lines.Last()), 'No');
        Check('details last', Format(Card.Details.Last()), 'No');
        Check('lines next', Format(Card.Lines.Next()), 'No');
        Check('details next', Format(Card.Details.Next()), 'No');
        Card.Close();
    end;

    [Test]
    procedure NoLinesOpenEditGoToKey()
    var
        Card: TestPage "PN Card";
    begin
        Setup();
        Card.OpenEdit();
        Card.GoToKey('H2');
        Check('lines first', Format(Card.Lines.First()), 'No');
        Check('details first', Format(Card.Details.First()), 'No');
        Check('lines value', Card.Lines."Line No.".Value(), '');
        Check('details value', Card.Details.Seq.Value(), '');
        Card.Close();
    end;
}
"#;

const MEASURED: [&str; 5] = [
    "NoLinesFirst",
    "NoLinesDetailsFirstOnly",
    "NoLinesNoMove",
    "NoLinesLastNext",
    "NoLinesOpenEditGoToKey",
];

#[test]
fn r7392_measured_cells_are_flagged_passes() {
    for test in MEASURED {
        let (o, marks) = outcome_and_marks(SRC, "PN Tests", test);
        assert_eq!(o, Outcome::Pass, "{test}");
        assert!(marks.contains(&ProviderNoRowExtChain), "{test}: {marks:?}");
    }
    let (o, marks) = outcome_and_marks(SRC, "PN Tests", "WithLinesFirst");
    assert_eq!(o, Outcome::Pass);
    assert!(!marks.contains(&ProviderNoRowExtChain), "{marks:?}");
}

fn variant(from: &str, to: &str) -> String {
    assert!(SRC.contains(from), "not in the source: {from}");
    SRC.replacen(from, to, 1)
}

/// One fact of a measured cell changed: the chain keeps main's refusal and takes no assumption. A call after the landing
/// that the probe does not make is refused by the end the landing set (the landing itself took the assumption).
#[test]
fn r7392_neighbors_keep_mains_refusal() {
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "a detail row holds the blank link values",
            variant(
                "        Detail.Insert();\n    end;",
                "        Detail.Insert();\n        Detail.\"Header No.\" := '';\n        Detail.\"Line No.\" := 0;\n        Detail.Insert();\n    end;",
            ),
            "NoLinesFirst",
        ),
        (
            "OpenEdit with GoToRecord",
            variant(
                "        Card.OpenView();\n        Card.GoToRecord(Header);\n        Check('lines first'",
                "        Card.OpenEdit();\n        Card.GoToRecord(Header);\n        Check('lines first'",
            ),
            "NoLinesFirst",
        ),
        (
            "OpenView with GoToKey",
            variant(
                "        Card.OpenEdit();\n        Card.GoToKey('H2');",
                "        Card.OpenView();\n        Card.GoToKey('H2');",
            ),
            "NoLinesOpenEditGoToKey",
        ),
        (
            "a host Next after the landing",
            variant(
                "        Check('details value', Card.Details.Seq.Value(), '');\n        Card.Close();\n    end;\n\n    [Test]\n    procedure WithLinesFirst",
                "        Card.Next();\n        Card.Close();\n    end;\n\n    [Test]\n    procedure WithLinesFirst",
            ),
            "NoLinesFirst",
        ),
        (
            "a trigger on the detail part",
            variant(
                "    layout { area(Content) { repeater(Rows) { field(Seq; Rec.Seq)",
                "    trigger OnAfterGetCurrRecord() begin end;\n    layout { area(Content) { repeater(Rows) { field(Seq; Rec.Seq)",
            ),
            "NoLinesFirst",
        ),
        (
            "a third link field",
            variant(
                "\"Line No.\" = field(\"Line No.\");",
                "\"Line No.\" = field(\"Line No.\"), Seq = const(1);",
            ),
            "NoLinesFirst",
        ),
    ];
    for (what, src, test) in cases {
        let (o, marks) = outcome_and_marks(&src, "PN Tests", test);
        if what.starts_with("a host Next") {
            assert!(
                matches!(&o, Outcome::NotRun(r) if r.contains("whose Provider part shows no row, other than")),
                "{what}: {o:?}"
            );
            continue;
        }
        assert!(
            matches!(&o, Outcome::NotRun(r) if r.contains("Provider part that shows no row")),
            "{what}: {o:?}"
        );
        assert!(!marks.contains(&ProviderNoRowExtChain), "{what}: {marks:?}");
    }
}
