//! `SubPageLink = F = field(filter(H))` and `RunPageLink` of that form: the host field's value is read as a filter text, a blank
//! value sets no filter (pages/lt-y37-link-filter/Y37LinkFilterTests.al).
use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 61980 "LF Line" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 61981 "LF Host" { fields { field(1; "No."; Code[20]) { } field(2; "Line Filter"; Text[250]) { } } keys { key(PK; "No.") { } } }
codeunit 61982 "LF Log"
{
    SingleInstance = true;
    var
        Buffer: Text;
    procedure Add(T: Text) begin Buffer += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Buffer; Buffer := ''; exit(R); end;
}
page 61983 "LF Part"
{
    PageType = ListPart; SourceTable = "LF Line"; Editable = false;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } } } }
    trigger OnAfterGetCurrRecord()
    var
        G: Integer;
    begin
        G := Rec.FilterGroup();
        Rec.FilterGroup(4);
        Log.Add(Rec."No." + ' ' + Rec.GetFilters());
        Rec.FilterGroup(G);
    end;
    var
        Log: Codeunit "LF Log";
}
page 61984 "LF Card"
{
    PageType = Card; SourceTable = "LF Host";
    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { }
            field(LineFilterCtl; Rec."Line Filter") { }
            part(LinesPart; "LF Part") { SubPageLink = "No." = field(filter("Line Filter")); }
        }
    }
    actions { area(Processing) { action(UpdateAction) { trigger OnAction() begin CurrPage.Update(); end; } } }
}
codeunit 61985 C
{
    Subtype = Test;

    var
        Log: Codeunit "LF Log";

    local procedure Seed(LineFilter: Text[250])
    var
        H: Record "LF Host";
        L: Record "LF Line";
    begin
        H.DeleteAll();
        L.DeleteAll();
        L."No." := 'A1';
        L.Insert();
        L."No." := 'A2';
        L.Insert();
        L."No." := 'B1';
        L.Insert();
        H."No." := 'H1';
        H."Line Filter" := LineFilter;
        H.Insert();
    end;

    local procedure Rows(var P: TestPage "LF Card"): Text
    var
        R: Text;
    begin
        if P.LinesPart.First() then
            repeat
                R += P.LinesPart.NoCtl.Value() + ',';
            until not P.LinesPart.Next();
        exit(R);
    end;

    local procedure Check(Name: Text; Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('%1: got [%2], want [%3]', Name, Got, Want);
    end;

    [Test]
    procedure FilterText()
    var
        P: TestPage "LF Card";
    begin
        Seed('A*|B1');
        P.OpenEdit();
        Check('rows', Rows(P), 'A1,A2,B1,');
        P.Close();
        Seed('');
        P.OpenEdit();
        Check('blank', Rows(P), 'A1,A2,B1,');
        P.Close();
    end;

    [Test]
    procedure ValueChangedThenUpdate()
    var
        P: TestPage "LF Card";
        Host: Record "LF Host";
    begin
        Seed('A1');
        P.OpenEdit();
        Log.Take();
        P.LineFilterCtl.SetValue('A*');
        Check('setvalue log', Log.Take(), 'A1 No.: A*;');
        Check('rows', Rows(P), 'A1,A2,');
        Log.Take();
        P.UpdateAction.Invoke();
        Check('update log', Log.Take(), 'A2 No.: A*;');
        Check('rows after update', Rows(P), 'A1,A2,');
        P.Close();
        Host.Get('H1');
        Check('stored', Host."Line Filter", 'A*');
    end;
}
"#;

#[test]
fn a_host_value_is_read_as_a_filter_text() {
    let got = outcomes_in(SRC, "C");
    assert!(matches!(got[0], (_, Outcome::Pass)), "{got:?}");
    // The part follows a SetValue of the host field at once; an action saves the edit first and the part reads its rows once, at
    // the action's CurrPage.Update (Y37LinkFilterTests.al:418 LT_C311_Link_ValueChangedOnPage).
    assert!(matches!(got[1], (_, Outcome::Pass)), "{got:?}");
}
