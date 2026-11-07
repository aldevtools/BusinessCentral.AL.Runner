//! A page that a TestPage opened closes itself with CurrPage.Close in OnAfterGetCurrRecord and then calls a procedure of its part:
//! the part answers, its globals as its OnClosePage left them. Mirrors corpus pages/parts/Y37PartAfterClose.al:652
//! CLP_AGCR_CloseAllowed_PartAccess_Observe and the fixture _fixtures/PagTriggerCloseParts.al (corpus PR 1205).

use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 62760 "PS Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 62761 "PS Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Mark(M: Text) begin T += M + ';'; end;
    procedure Take(): Text var R: Text; begin R := T; T := ''; exit(R); end;
}
page 62762 "PS Part"
{
    PageType = ListPart; SourceTable = "PS Row"; SourceTableTemporary = true;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } } } }
    var Log: Codeunit "PS Log"; State: Integer;
    trigger OnOpenPage() begin State := 41; Log.Mark('PART.OPEN'); end;
    trigger OnClosePage() begin State += 1000; Log.Mark('PART.CLOSE'); end;
    procedure Touch(): Text begin State += 1; exit(Format(State)); end;
}
page 62763 "PS Parent"
{
    PageType = Card; SourceTable = "PS Row"; SourceTableTemporary = true;
    layout { area(Content) { field(Id; Rec.Id) { } part(PartBody; "PS Part") { } } }
    var Log: Codeunit "PS Log"; Probed: Boolean;
    trigger OnOpenPage() begin Log.Mark('PARENT.OPEN'); Rec.Id := 1; Rec.Insert(); end;
    trigger OnAfterGetCurrRecord()
    begin
        if Probed then exit;
        Probed := true;
        CurrPage.Close();
        Log.Mark('AFTER.CLOSE');
        Log.Mark('PART.TOUCH:' + CurrPage.PartBody.Page.Touch());
    end;
    trigger OnClosePage() begin Log.Mark('PARENT.CLOSE'); end;
}
codeunit 62790 T
{
    Subtype = Test;
    var Log: Codeunit "PS Log";
    [Test] procedure Run()
    var P: TestPage "PS Parent"; L: Text;
    begin
        Log.Take();
        P.OpenView();
        L := Log.Take();
        if L <> 'PARENT.OPEN;PART.OPEN;PART.CLOSE;PARENT.CLOSE;AFTER.CLOSE;PART.TOUCH:1042;' then Error('%1', L);
    end;
}
"#;

#[test]
fn a_part_answers_after_its_page_closed_itself() {
    let o = al_testkit::outcomes_in(SRC, "T");
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0].1);
}
