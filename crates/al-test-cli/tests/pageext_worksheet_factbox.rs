//! A page extension adds a part to `factboxes` of a Worksheet page that declares no FactBoxes area, and the extension's
//! OnAfterGetCurrRecord loads the hidden part (al-runner issue 6490, the reporter's app).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 50610 "PFW T"
{
    fields { field(1; K; Code[10]) { } }
    keys { key(PK; K) { Clustered = true; } }
}
page 50611 "PFW Box"
{
    PageType = CardPart;
    SourceTable = "PFW T";
    layout { area(Content) { field(K; Rec.K) { } } }
    procedure LoadData(R: Record "PFW T")
    begin
        Loaded := R.K;
    end;
    procedure GetLoaded(): Code[10]
    begin
        exit(Loaded);
    end;
    var
        Loaded: Code[10];
}
page 50612 "PFW Sheet"
{
    PageType = Worksheet;
    SourceTable = "PFW T";
    layout { area(Content) { repeater(Lines) { field(K; Rec.K) { } } } }
}
pageextension 50613 "PFW Sheet Ext" extends "PFW Sheet"
{
    layout
    {
        addfirst(factboxes)
        {
            part(Box; "PFW Box") { Visible = false; }
        }
    }
    trigger OnAfterGetCurrRecord()
    begin
        CurrPage.Box.Page.LoadData(Rec);
    end;
}
codeunit 50614 "T"
{
    Subtype = Test;
    [Test]
    procedure WorksheetWithAddedFactbox()
    var R: Record "PFW T"; TP: TestPage "PFW Sheet";
    begin
        R.K := 'A';
        R.Insert();
        TP.OpenView();
        if TP.K.Value <> 'A' then Error('k %1', TP.K.Value);
        // The part added with Visible = false is not on the test page (corpus writer Y20, Y20ImplicitFactBoxes.al:43-51).
        asserterror TP.Box.K.Value();
        if GetLastErrorCode() <> 'TestPartNotFound' then Error('hidden part: %1 %2', GetLastErrorCode(), GetLastErrorText());
    end;
}
"#;

#[test]
fn a_worksheet_takes_an_added_factbox() {
    let out = outcomes_in(SRC, "T");
    assert_eq!(out, vec![("WorksheetWithAddedFactbox".to_string(), Outcome::Pass)]);
}
