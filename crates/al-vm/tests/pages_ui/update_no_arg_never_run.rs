//! `Update()` without an argument on a Page variable that never ran does nothing, from its own procedure (also on a page
//! with no source table) and from outside. corpus pages/lt-y50-update-never-run/Y50UpdateNeverRun.al (G5-16).

use al_testkit::text;

const SRC: &str = r#"
table 50310 "UNR Row"
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

page 50311 "UNR Navigate"
{
    PageType = NavigatePage;
    layout
    {
        area(Content)
        {
            field(ShownCtl; Shown) { }
        }
    }
    var
        Shown: Text;

    procedure UpdateOnly(NewText: Text): Text
    begin
        Shown := NewText;
        CurrPage.Update();
        exit('after ' + Shown);
    end;
}

page 50312 "UNR Card"
{
    PageType = Card;
    SourceTable = "UNR Row";
    layout
    {
        area(Content)
        {
            field(IdCtl; Rec.Id) { }
        }
    }

    procedure SetFilterAndUpdate(F: Text): Text
    begin
        Rec.SetFilter(Id, F);
        CurrPage.Update();
        exit('after ' + Rec.GetFilters());
    end;
}

codeunit 50313 "UNR Test"
{
    procedure Run(): Text
    var
        N: Page "UNR Navigate";
        C: Page "UNR Card";
        D: Page "UNR Card";
    begin
        D.Update();
        exit(N.UpdateOnly('hello') + '|' + C.SetFilterAndUpdate('2') + '|done');
    end;
}
"#;

#[test]
fn update_without_argument_on_a_page_variable_that_never_ran() {
    assert_eq!(text(al_testkit::run_in(SRC, "UNR Test", "Run")), "after hello|after Id: 2|done");
}
