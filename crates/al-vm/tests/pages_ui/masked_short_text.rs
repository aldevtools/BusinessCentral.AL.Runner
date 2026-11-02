//! A Masked control shows ten bullets for any non-empty text, whatever its length, on a field and on a page variable
//! (corpus pages/lt-y72-page-probes/Y72MaskedShortText.al).

use al_testkit::text;

const SRC: &str = r#"
table 69960 "MS Row" { fields { field(1; K; Integer) { } field(2; Secret; Text[50]) { } } keys { key(PK; K) { } } }
page 69961 "MS Card"
{
    PageType = Card; SourceTable = "MS Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(SecretF; Rec.Secret) { ExtendedDatatype = Masked; }
            field(VarF; PwVar) { ExtendedDatatype = Masked; }
        }
    }
    var PwVar: Text;
    trigger OnAfterGetRecord() begin PwVar := '***'; end;
}
codeunit 69962 C
{
    procedure Short(): Text
    var R: Record "MS Row"; P: TestPage "MS Card";
    begin
        R.K := 1; R.Secret := 'a'; R.Insert();
        P.OpenView();
        exit(P.SecretF.Value() + '|' + P.VarF.Value() + '|' + Format(StrLen(P.VarF.Value())));
    end;
}
"#;

#[test]
fn a_masked_control_shows_ten_bullets_for_a_short_text() {
    // Y72MaskedShortText.al Masked_LengthSweep_Field (length 1) and Masked_PageVariable_Short ('***').
    let ten = "\u{2022}".repeat(10);
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Short")), format!("{ten}|{ten}|10"));
}
