//! After a Page variable's RunModal closed with OK, the page's own code may still reach a part a Visible expression had hidden
//! (it hands out its row at close) and call CurrPage.Update and SaveRecord, which do nothing (pages/lt-y37-part-after-close/
//! Y37PartAfterClose.al:376-441).
use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 61970 "PC Vend" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 61971 "PC Line" { fields { field(1; Vend; Code[20]) { } field(2; "Line No."; Integer) { } } keys { key(PK; Vend, "Line No.") { } } }
page 61972 "PC Part"
{
    PageType = ListPart; SourceTable = "PC Line"; Editable = false;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } } } }
    procedure LineNo(): Integer begin exit(Rec."Line No."); end;
    procedure PartUpdate() begin CurrPage.Update(false); end;
}
page 61973 "PC Dialog"
{
    PageType = Card; SourceTable = "PC Vend"; InsertAllowed = false; DeleteAllowed = false;
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { Editable = false; }
            field(Show; Show) { }
            part(PartA; "PC Part") { SubPageLink = Vend = field("No."); Visible = Show; }
        }
    }
    trigger OnOpenPage() begin Show := true; end;
    var
        Show: Boolean;
    procedure PartLine(): Integer begin exit(CurrPage.PartA.Page.LineNo()); end;
    procedure HostUpdate() begin CurrPage.Update(true); CurrPage.SaveRecord(); CurrPage.PartA.Page.PartUpdate(); end;
}
codeunit 61974 C
{
    Subtype = Test;

    [Test]
    [HandlerFunctions('H')]
    procedure AfterClose()
    var
        V: Record "PC Vend";
        L: Record "PC Line";
        Pg: Page "PC Dialog";
    begin
        V."No." := 'V1';
        V.Insert();
        L.Vend := 'V1';
        L."Line No." := 10000;
        L.Insert();
        L."Line No." := 20000;
        L.Insert();
        Pg.SetRecord(V);
        Pg.LookupMode := true;
        Pg.RunModal();
        Pg.HostUpdate();
        if Pg.PartLine() <> 10000 then
            Error('part line %1', Pg.PartLine());
    end;

    [ModalPageHandler]
    procedure H(var P: TestPage "PC Dialog")
    begin
        P.Show.SetValue(false);
        P.OK().Invoke();
    end;
}
"#;

#[test]
fn a_part_hidden_before_close_and_update_after_close_are_served() {
    let got = outcomes_in(SRC, "C");
    assert!(matches!(got.as_slice(), [(_, Outcome::Pass)]), "{got:?}");
}
