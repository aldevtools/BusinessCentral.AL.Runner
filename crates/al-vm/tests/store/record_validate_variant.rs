//! `Record.Validate(Field, V)` with `V` a Variant converts the value into the field as `FieldRef.Validate` does: a Variant holding an
//! Enum, an Option, a number or a Duration goes into a Text or Code field as its formatted text (al-runner issue 6975; corpus
//! record/validate/RecordValidateVariant.al), where an assignment of the Variant raises.

const SRC: &str = r#"
enum 50985 "Vv Kind" { value(0; One) { } value(1; Two) { } }
table 50986 "Vv Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Txt; Text[30]) { trigger OnValidate() begin Hits += 1; end; }
        field(3; Cod; Code[20]) { trigger OnValidate() begin Hits += 1; end; }
        field(4; Hits; Integer) { }
    }
    keys { key(PK; Id) { } }
}
codeunit 50987 C
{
    local procedure Into(var R: Record "Vv Row"; V: Variant)
    begin
        R.Validate(Txt, V);
        R.Validate(Cod, V);
    end;

    procedure F(): Text
    var R: Record "Vv Row"; Kind: Enum "Vv Kind"; Op: Option Alpha,Beta; T: Text;
    begin
        Kind := Kind::Two;
        Into(R, Kind);
        T := R.Txt + '/' + R.Cod + '/' + Format(R.Hits);
        Op := Op::Beta;
        Into(R, Op);
        T += '|' + R.Txt + '/' + R.Cod;
        Into(R, 42);
        T += '|' + R.Txt + '/' + R.Cod;
        Into(R, true);
        T += '|' + R.Txt + '/' + R.Cod;
        exit(T);
    end;
}
"#;

#[test]
fn record_validate_converts_a_variant_like_fieldref_validate() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "F")),
        "Two/TWO/2|Beta/BETA|42/42|Yes/YES"
    );
}
