//! Format of a TestPage control is the text the control shows, cut or padded to the length; the format number changes
//! nothing (corpus errors/lt-ms-refusals-132/P132TestPageFormatRisky.al:P132_FormatOfControl_Standard0, _StandardNumbers;
//! errors/lt-ms-refusals-129/P129FormatTestPageControl.al:94-138).

const SRC: &str = r#"
table 69980 "FmtCtl Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Dec; Decimal) { }
        field(3; Txt; Text[30]) { }
        field(4; Flag; Boolean) { }
        field(5; Opt; Option) { OptionMembers = Zero,One,Two; OptionCaption = 'Nil,Uno,Dos'; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
page 69981 "FmtCtl Card"
{
    PageType = Card; SourceTable = "FmtCtl Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(IdCtl; Rec.Id) { } field(DecCtl; Rec.Dec) { } field(TxtCtl; Rec.Txt) { } field(FlagCtl; Rec.Flag) { } field(OptCtl; Rec.Opt) { } } }
}
codeunit 69982 C
{
    procedure Run(): Text
    var
        R: Record "FmtCtl Row";
        P: TestPage "FmtCtl Card";
        T: Text;
    begin
        R.Id := 1234;
        R.Dec := 1234.5;
        R.Txt := 'Hello world';
        R.Flag := true;
        R.Opt := R.Opt::Two;
        R.Insert();
        P.OpenView();
        T := Format(P.IdCtl) + '|' + Format(P.IdCtl, 0, 2) + '|' + Format(P.IdCtl, 8) + '|' + Format(P.DecCtl, 0, 9) + '|';
        T += Format(P.TxtCtl, 5) + '|' + Format(P.FlagCtl, 0, 9) + '|' + Format(P.OptCtl, 0, 2);
        P.Close();
        exit(T);
    end;
}
"#;

#[test]
fn a_test_page_control_formats_as_the_text_it_shows() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Run")),
        "1234|1234|1234    |1,234.50|Hello|Yes|Dos"
    );
}
