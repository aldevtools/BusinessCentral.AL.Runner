//! A conditional with a Label branch handed to a Variant: the Variant holds the kind of the branch taken. A Label before the
//! colon against another Label or against a Text or Code that is no literal gives a TextConstant when the Label branch is
//! taken; with a literal, or with the Label second, the Variant holds Text or Code (writer Y81,
//! types/lt-y81-values/Y81LabelVariant.al routes 1 to 13; needs corpus bump).

use al_builtins::value::Value;
use al_testkit::{run_with, text};

const SRC: &str = r#"
codeunit 69974 C
{
    var
        LblA: Label 'Label A';
        LblB: Label 'Label B';
        TxtVar: Text;
        CodeVar: Code[20];

    local procedure Kinds(V: Variant): Text
    begin
        exit(Format(V.IsText()) + '/' + Format(V.IsCode()) + '/' + Format(V.IsTextConstant()) + '/' + Format(V.IsChar()) + '=' + Format(V) + '|' + Format(StrLen(Take(V))));
    end;

    local procedure Take(V: Variant): Text
    var T: Text;
    begin
        T := V;
        exit(T);
    end;

    local procedure ReturnLiteralOrLabel(Cond: Boolean): Variant
    begin
        exit(Cond ? 'literal' : LblA);
    end;

    local procedure ReturnLabels(Cond: Boolean): Variant
    begin
        exit(Cond ? LblA : LblB);
    end;

    procedure Route(Cond: Boolean; R: Integer): Text
    var
        V: Variant;
        Arr: array[2] of Variant;
    begin
        TxtVar := 'text var';
        CodeVar := 'code var';
        case R of
            1: V := Cond ? LblA : LblB;
            2: V := Cond ? LblA : 'literal';
            3: V := Cond ? 'literal' : LblA;
            4: V := Cond ? LblA : TxtVar;
            5: V := Cond ? TxtVar : LblA;
            6: V := Cond ? LblA : CodeVar;
            7: V := Cond ? CodeVar : LblA;
            8: exit(Kinds(Cond ? LblA : LblB));
            11: V := ReturnLiteralOrLabel(Cond);
            12: V := ReturnLabels(Cond);
            13: begin Arr[2] := Cond ? LblA : LblB; V := Arr[2]; end;
        end;
        exit(Kinds(V));
    end;
}
"#;

fn route(cond: bool, r: i32) -> String {
    text(run_with(SRC, "C", "Route", vec![Value::Boolean(cond), Value::Integer(r)]).map(|x| x.0))
}

#[test]
fn the_variant_kind_follows_the_branch_taken() {
    let tc = |s: &str, n: usize| format!("No/No/Yes/No={s}|{n}");
    let t = |s: &str, n: usize| format!("Yes/No/No/No={s}|{n}");
    let c = |s: &str, n: usize| format!("No/Yes/No/No={s}|{n}");
    assert_eq!(route(true, 1), tc("Label A", 7));
    assert_eq!(route(false, 1), tc("Label B", 7));
    assert_eq!(route(true, 2), t("Label A", 7));
    assert_eq!(route(false, 2), t("literal", 7));
    assert_eq!(route(true, 3), t("literal", 7));
    assert_eq!(route(false, 3), t("Label A", 7));
    assert_eq!(route(true, 4), tc("Label A", 7));
    assert_eq!(route(false, 4), t("text var", 8));
    assert_eq!(route(true, 5), t("text var", 8));
    assert_eq!(route(false, 5), t("Label A", 7));
    assert_eq!(route(true, 6), tc("Label A", 7));
    assert_eq!(route(false, 6), c("CODE VAR", 8));
    assert_eq!(route(true, 7), c("CODE VAR", 8));
    assert_eq!(route(false, 7), c("LABEL A", 7));
    assert_eq!(route(true, 8), tc("Label A", 7));
    assert_eq!(route(false, 8), tc("Label B", 7));
    assert_eq!(route(true, 11), t("literal", 7));
    assert_eq!(route(false, 11), t("Label A", 7));
    assert_eq!(route(true, 12), tc("Label A", 7));
    assert_eq!(route(true, 13), tc("Label A", 7));
}
