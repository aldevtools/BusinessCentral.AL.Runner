//! CaptionClass = '3,' with nothing after the comma on a page control (pages/properties/Y80CaptionClassEmpty.al,
//! same on BC 27, 28 and 29): the control shows the caption of the LAST other control on the same field (its Caption, else the
//! field's caption); the only control on its field shows its own Caption.

const SRC: &str = r#"
table 69960 "CE Rec"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Plain; Text[30]) { Caption = 'Plain Field'; }
        field(3; Other; Text[30]) { Caption = 'Other Field'; }
        field(4; Third; Text[30]) { Caption = 'Third Field'; }
    }
    keys { key(PK; Id) { } }
}
page 69961 "CE Card"
{
    PageType = Card; SourceTable = "CE Rec"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { group(G) {
        field(CtlA; Rec.Plain) { Caption = 'Ctl A'; }
        field(CtlLit; Rec.Plain) { Caption = 'Own Lit'; CaptionClass = '3,'; }
        field(CtlB; Rec.Plain) { Caption = 'Ctl B'; }
        field(CtlExpr; Rec.Other) { Caption = 'Own Expr'; CaptionClass = Cap3(); }
        field(CtlOtherNoCaption; Rec.Other) { }
        field(CtlAlone; Rec.Third) { Caption = 'Own Alone'; CaptionClass = '3,'; }
    } } }
    local procedure Cap3(): Text begin exit('3,'); end;
}
codeunit 69962 C
{
    procedure Caps(): Text
    var R: Record "CE Rec"; P: TestPage "CE Card"; T: Text;
    begin
        R.Id := 1; R.Insert();
        P.OpenEdit();
        T := P.CtlA.Caption() + '|' + P.CtlLit.Caption() + '|' + P.CtlB.Caption() + '|' + P.CtlExpr.Caption() + '|'
            + P.CtlOtherNoCaption.Caption() + '|' + P.CtlAlone.Caption();
        P.Close();
        exit(T);
    end;
}
"#;

#[test]
fn an_empty_three_class_reads_the_last_other_control_on_the_field() {
    let mut built = al_testkit::Built::from_source(SRC);
    assert_eq!(
        al_testkit::text(built.run_in("C", "Caps")),
        "Ctl A|Ctl B|Ctl B|Other Field|Other Field|Own Alone"
    );
}
