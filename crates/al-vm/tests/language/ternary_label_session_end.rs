//! A conditional `Cond ? Label : 'literal'` (Label first, text literal second) used as the exit value of a procedure that
//! returns a Variant, or as the argument of a Variant parameter, ends the whole BC 29 session when its codeunit is loaded,
//! even if the procedure is never called (writer Y81, types/lt-y81-values/Y81LabelVariant.al codeunit 94259 "LT Y81 Label
//! Variant Held", Suspect_ReturnLabelOrLiteral and Suspect_ArgumentLabelOrLiteral; needs corpus bump). The runtime refuses
//! both shapes statically and names the shape.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::run_with;

const SRC: &str = r#"
codeunit 69970 Held
{
    var
        LblA: Label 'Label A';

    procedure Suspect_ReturnLabelOrLiteral(Cond: Boolean): Variant
    begin
        exit(Cond ? LblA : 'literal');
    end;

    procedure Suspect_ArgumentLabelOrLiteral(Cond: Boolean): Text
    begin
        exit(Describe(Cond ? LblA : 'literal'));
    end;

    procedure Fine_LiteralFirst(Cond: Boolean): Text
    begin
        exit(Describe(Cond ? 'literal' : LblA));
    end;

    procedure Fine_TextArgument(Cond: Boolean): Text
    begin
        exit(DescribeText(Cond ? LblA : 'literal'));
    end;

    local procedure Describe(V: Variant): Text
    begin
        exit(Format(V.IsText()));
    end;

    local procedure DescribeText(V: Text): Text
    begin
        exit(V);
    end;
}
"#;

fn reason(routine: &str) -> String {
    match run_with(SRC, "Held", routine, vec![Value::Boolean(true)]) {
        Err(Signal::Unsupported(s)) => s,
        other => format!("not refused: {other:?}"),
    }
}

#[test]
fn return_of_label_or_literal_in_a_variant_function_is_refused() {
    let r = reason("Suspect_ReturnLabelOrLiteral");
    assert!(r.contains("BC 29 ends the session"), "{r}");
}

#[test]
fn variant_argument_of_label_or_literal_is_refused() {
    let r = reason("Suspect_ArgumentLabelOrLiteral");
    assert!(r.contains("BC 29 ends the session"), "{r}");
}

#[test]
fn the_other_shapes_are_not_refused_for_this_reason() {
    for name in ["Fine_LiteralFirst", "Fine_TextArgument"] {
        let r = reason(name);
        assert!(!r.contains("ends the session"), "{name}: {r}");
    }
}
