//! al-runner issue 6877 (follow-up of 6038): a ListPart with UpdatePropagation = Both that stands on a stored row other than
//! its first and runs CurrPage.Update(false) in a control's OnValidate. Nothing pins which row it shows afterwards (the issue's
//! reduced app, Document, Card and ListPlus host), so it is refused, as the same step without Both is on a Card or ListPlus host.
//! The first row and the one-row parts stay as pinned (QjPartPropagation.al:1224, :2156, :2212).

use al_builtins::native::Signal;

fn src(page_type: &str) -> String {
    r#"
table 50100 "Demo Header" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 50101 "Demo Line"
{
    fields { field(1; "Header No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Code; Code[20]) { } field(4; Description; Text[50]) { } }
    keys { key(PK; "Header No.", "Line No.") { Clustered = true; } }
}
page 50100 "Demo Host"
{
    PageType = PAGETYPE;
    SourceTable = "Demo Header";
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "Demo Lines Part") { SubPageLink = "Header No." = field("No."); UpdatePropagation = Both; } } }
}
page 50101 "Demo Lines Part"
{
    PageType = ListPart;
    SourceTable = "Demo Line";
    InsertAllowed = false;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { }
                field("Code"; Rec.Code) { trigger OnValidate() begin CurrPage.Update(false); end; }
                field(Description; Rec.Description) { }
            }
        }
    }
}
codeunit 50100 "Demo Part Tests"
{
    local procedure CreateDoc(var Header: Record "Demo Header")
    var Line: Record "Demo Line";
    begin
        Header."No." := 'H1'; Header.Insert();
        Line."Header No." := 'H1'; Line."Line No." := 10000; Line.Insert();
        Line."Line No." := 20000; Line.Insert();
        Line."Line No." := 30000; Line.Insert();
    end;

    procedure RowAfter(Skip: Integer): Integer
    var Header: Record "Demo Header"; Host: TestPage "Demo Host"; i: Integer;
    begin
        CreateDoc(Header);
        Host.OpenEdit();
        Host.GoToRecord(Header);
        Host.Lines.First();
        for i := 1 to Skip do Host.Lines.Next();
        Host.Lines.Code.SetValue('X');
        exit(Host.Lines."Line No.".AsInteger());
    end;
}
"#
    .replace("PAGETYPE", page_type)
}

fn row_after(page_type: &str, skip: i32) -> Result<i32, Signal> {
    let s = src(page_type);
    let mut b = al_testkit::Built::from_source(&s);
    let (oi, ri) = b.routine_in("Demo Part Tests", "RowAfter");
    b.call(oi, ri, vec![al_builtins::value::Value::Integer(skip)])
        .map(|(v, _)| al_testkit::int(Ok(v)))
}

#[test]
fn a_later_row_with_both_is_refused_on_every_host_type() {
    for t in ["Document", "Card", "ListPlus"] {
        for skip in [1, 2] {
            let r = row_after(t, skip);
            assert!(matches!(&r, Err(Signal::Unsupported(_))), "{t} row {skip}: {r:?}");
        }
    }
}

#[test]
fn the_first_row_with_both_still_runs() {
    for t in ["Document", "Card", "ListPlus"] {
        assert_eq!(row_after(t, 0).unwrap(), 10000, "{t}");
    }
}
