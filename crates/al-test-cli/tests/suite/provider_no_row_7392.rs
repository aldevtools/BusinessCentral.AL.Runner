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
    edit(SRC, from, to)
}

fn edit(src: &str, from: &str, to: &str) -> String {
    assert!(src.contains(from), "not in the source: {from}");
    src.replacen(from, to, 1)
}

const DETAIL_FIELD: &str = "field(Seq; Rec.Seq) { ApplicationArea = All; }";
const EXT_HEAD: &str = "pageextension 69815 \"PN Card Ext\" extends \"PN Card\"\n{\n";
const EXT_LINK: &str = "SubPageLink = \"Header No.\" = field(\"Header No.\"), \"Line No.\" = field(\"Line No.\");";

/// One fact of a measured cell changed: the chain keeps main's refusal and takes no assumption.
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
                EXT_LINK,
                "SubPageLink = \"Header No.\" = field(\"Header No.\"), \"Line No.\" = field(\"Line No.\"), Seq = const(1);",
            ),
            "NoLinesFirst",
        ),
        (
            // A blank Code value does not convert to Integer: main refused before converting it.
            "a link entry from a Code field to an Integer field",
            variant(
                EXT_LINK,
                "SubPageLink = \"Header No.\" = field(\"Header No.\"), Seq = field(\"Header No.\");",
            )
            .replace("'H1'", "'1'")
            .replace("'H2'", "'2'"),
            "NoLinesFirst",
        ),
        (
            "the link entries in another order",
            variant(
                EXT_LINK,
                "SubPageLink = \"Line No.\" = field(\"Line No.\"), \"Header No.\" = field(\"Header No.\");",
            ),
            "NoLinesFirst",
        ),
        (
            "the extension declares a property",
            variant(EXT_HEAD, &format!("{EXT_HEAD}    Editable = false;\n")),
            "NoLinesFirst",
        ),
        (
            "the extension declares a variable and a procedure",
            variant(
                "}\ncodeunit 69816",
                "    var X: Integer;\n    procedure P() begin X := 1; end;\n}\ncodeunit 69816",
            ),
            "NoLinesFirst",
        ),
        (
            "the extension modifies a field of the Card",
            variant(
                "        addafter(Lines)\n",
                "        modify(\"No.\") { Visible = false; }\n        addafter(Lines)\n",
            ),
            "NoLinesFirst",
        ),
        (
            "the extension adds the part before the Provider part",
            variant("        addafter(Lines)\n", "        addbefore(Lines)\n"),
            "NoLinesFirst",
        ),
        (
            "the Card declares a variable and a procedure",
            variant(
                "}\npage 69812",
                "    var X: Integer;\n    procedure P() begin X := 1; end;\n}\npage 69812",
            ),
            "NoLinesFirst",
        ),
        (
            "a control trigger on the detail part",
            variant(
                DETAIL_FIELD,
                "field(Seq; Rec.Seq) { ApplicationArea = All; trigger OnDrillDown() begin end; }",
            ),
            "NoLinesFirst",
        ),
        (
            "an action on the detail part",
            variant(
                "}\npageextension 69815",
                "    actions { area(Processing) { action(A) { ApplicationArea = All; trigger OnAction() begin end; } } }\n}\npageextension 69815",
            ),
            "NoLinesFirst",
        ),
        (
            "an expression control on the detail part",
            variant(
                DETAIL_FIELD,
                &format!("{DETAIL_FIELD} field(Calc; Rec.Seq + 1) {{ ApplicationArea = All; }}"),
            ),
            "NoLinesFirst",
        ),
        (
            "a Caption on the detail part's field",
            variant(
                DETAIL_FIELD,
                "field(Seq; Rec.Seq) { ApplicationArea = All; Caption = 'S'; }",
            ),
            "NoLinesFirst",
        ),
        (
            "the detail table declares a property",
            variant(
                "table 69813 \"PN Detail\"\n{\n",
                "table 69813 \"PN Detail\"\n{\n    Caption = 'D';\n",
            ),
            "NoLinesFirst",
        ),
    ];
    let wrong: Vec<String> = cases
        .into_iter()
        .filter_map(|(what, src, test)| {
            let (o, marks) = outcome_and_marks(&src, "PN Tests", test);
            let refused = matches!(&o, Outcome::NotRun(r) if r.contains("Provider part that shows no row"));
            (!refused || marks.contains(&ProviderNoRowExtChain)).then(|| format!("{what}: {o:?} {marks:?}"))
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// A call after the landing that the probe does not make is refused by the end the landing set (the landing itself took the
/// assumption): a host move, a read other than a part control's value, and any call after the test wrote to the store.
#[test]
fn r7392_calls_after_the_landing_keep_to_the_probe() {
    let after_first_only =
        "        Card.GoToRecord(Header);\n        Check('details first', Format(Card.Details.First()), 'No');";
    let first_only_var = "    procedure NoLinesDetailsFirstOnly()\n    var\n";
    let after_lines_first =
        "        Card.GoToRecord(Header);\n        Check('lines first', Format(Card.Lines.First()), 'No');";
    let lines_first_var = "    procedure NoLinesFirst()\n    var\n";
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "a host Next",
            variant(
                "        Check('details value', Card.Details.Seq.Value(), '');\n        Card.Close();\n    end;\n\n    [Test]\n    procedure WithLinesFirst",
                "        Card.Next();\n        Card.Close();\n    end;\n\n    [Test]\n    procedure WithLinesFirst",
            ),
            "NoLinesFirst",
        ),
        (
            "a detail row with the blank link values inserted after the landing",
            edit(
                &variant(
                    first_only_var,
                    &format!("{first_only_var}        Detail: Record \"PN Detail\";\n"),
                ),
                after_first_only,
                "        Card.GoToRecord(Header);\n        Detail.\"Seq\" := 7;\n        Detail.Insert();\n        Check('details first', Format(Card.Details.First()), 'No');",
            ),
            "NoLinesDetailsFirstOnly",
        ),
        (
            "a line inserted for the header after the landing",
            edit(
                &variant(
                    lines_first_var,
                    &format!("{lines_first_var}        Line: Record \"PN Line\";\n"),
                ),
                after_lines_first,
                "        Card.GoToRecord(Header);\n        Line.\"Header No.\" := 'H2';\n        Line.\"Line No.\" := 20000;\n        Line.Insert();\n        Check('lines first', Format(Card.Lines.First()), 'No');",
            ),
            "NoLinesFirst",
        ),
        (
            "AsInteger of a part control",
            variant(
                after_first_only,
                "        Card.GoToRecord(Header);\n        Card.Details.Seq.AsInteger();\n        Check('details first', Format(Card.Details.First()), 'No');",
            ),
            "NoLinesDetailsFirstOnly",
        ),
        (
            "a read of the Card's own field",
            variant(
                after_first_only,
                "        Card.GoToRecord(Header);\n        Check('no', Card.\"No.\".Value(), 'H2');\n        Check('details first', Format(Card.Details.First()), 'No');",
            ),
            "NoLinesDetailsFirstOnly",
        ),
    ];
    let wrong: Vec<String> = cases
        .into_iter()
        .filter_map(|(what, src, test)| {
            let (o, _) = outcome_and_marks(&src, "PN Tests", test);
            let refused = matches!(&o, Outcome::NotRun(r) if r.contains("whose Provider part shows no row"));
            (!refused).then(|| format!("{what}: {o:?}"))
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}
