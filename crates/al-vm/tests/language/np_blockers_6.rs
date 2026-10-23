fn text(vars: &str, body: &str) -> String {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end; }}");
    match al_testkit::run_in(&src, "C", "F") {
        Ok(al_builtins::value::Value::Text(t) | al_builtins::value::Value::Code(t)) => t.to_string(),
        Err(al_builtins::native::Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        Err(al_builtins::native::Signal::Unsupported(w)) => format!("unsupported:{w}"),
        other => format!("{other:?}"),
    }
}

// types/lt-variant-time-enum/VariantTextTimeEnumFilter.al:Variant_Text_ToDecimal
#[test]
fn a_text_variant_converts_into_a_decimal_by_the_pinned_forms() {
    for (input, want) in [
        ("-111.69", "-111.69"),
        ("5", "5"),
        ("1,000.5", "1000.5"),
        (" 7 ", "7"),
        ("100,5", "1005"),
    ] {
        let r = text(
            "V: Variant; T: Text; D: Decimal;",
            &format!("T := '{input}'; V := T; D := V; exit(Format(D, 0, 9));"),
        );
        assert_eq!(r, want, "input {input}");
    }
}

#[test]
fn a_text_variant_that_is_no_number_raises_the_type_conversion() {
    for input in ["", "x", "1e3"] {
        let r = text(
            "V: Variant; T: Text; D: Decimal;",
            &format!("T := '{input}'; V := T; D := V; exit('no error');"),
        );
        assert_eq!(
            r, "error:The expression Variant cannot be type-converted to a Decimal value.|TypeConversion:Format",
            "input {input:?}"
        );
    }
}

// types/lt-variant-time-enum/VariantTextTimeEnumFilter.al:Variant_CodeAndOtherTypes_ToDecimal `boolean`
#[test]
fn a_true_variant_converts_into_a_decimal_as_one() {
    let r = text(
        "V: Variant; B: Boolean; D: Decimal;",
        "B := true; V := B; D := V; exit(Format(D, 0, 9));",
    );
    assert_eq!(r, "1");
}

// json/JsonValueGetters.al:NumericTextGetters `numtext_asdecimal`, TextGetters `text_asdecimal`
#[test]
fn a_json_text_value_reads_as_a_decimal_when_it_is_digits() {
    let r = text(
        "J: JsonValue; D: Decimal;",
        "J.SetValue('42'); D := J.AsDecimal(); exit(Format(D, 0, 9));",
    );
    assert_eq!(r, "42");
    let r = text(
        "J: JsonValue; D: Decimal;",
        "J.SetValue('hello'); D := J.AsDecimal(); exit('no error');",
    );
    assert!(r.starts_with("error:Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavJsonValue to Microsoft.Dynamics.Nav.Runtime.Decimal18."), "{r}");
}
