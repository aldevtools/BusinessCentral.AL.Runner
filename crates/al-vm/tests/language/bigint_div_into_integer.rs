//! A BigInteger expression assigned to an Integer converts to Integer (al-runner issue 6660): the Variant it is passed
//! as then holds an Integer, not a BigInteger. `:=` of a BigInteger into an Integer converts
//! (types/lt-y31-conversion-matrix/Y31ConvCv.al:962-971 `BigInteger_To_Integer`).

use al_testkit::{run, text};

fn src(body: &str) -> String {
    format!(
        "codeunit 50100 C {{
    local procedure Ms(): BigInteger begin exit(86400000L); end;
    local procedure Kind(V: Variant): Text begin exit(Format(V.IsInteger()) + '/' + Format(V.IsBigInteger())); end;
    local procedure ByVar(var D: Integer) var Per: Integer; begin Per := 86400000; D := Ms() div Per; end;
    {body}
}}"
    )
}

#[test]
fn bigint_div_into_var_integer_parameter_is_an_integer() {
    let s = src("procedure F(): Text var D: Integer; begin ByVar(D); exit(Kind(D)); end;");
    assert_eq!(text(run(&s, "F")), "Yes/No");
}

#[test]
fn bigint_div_into_local_integer_is_an_integer() {
    let s = src(
        "procedure F(): Text var D: Integer; Per: Integer; begin Per := 86400000; D := Ms() div Per; exit(Kind(D)); end;",
    );
    assert_eq!(text(run(&s, "F")), "Yes/No");
}

#[test]
fn bigint_minus_into_integer_is_an_integer() {
    let s = src(
        "procedure F(): Text var D: Integer; Per: Integer; begin Per := 86400000; D := 1; D := (Ms() div Per) - D; exit(Kind(D)); end;",
    );
    assert_eq!(text(run(&s, "F")), "Yes/No");
}

#[test]
fn bigint_call_into_integer_is_an_integer() {
    let s = src("procedure F(): Text var D: Integer; begin D := Ms(); exit(Kind(D)); end;");
    assert_eq!(text(run(&s, "F")), "Yes/No");
}

#[test]
fn bigint_mod_into_integer_is_an_integer() {
    let s = src("procedure F(): Text var D: Integer; begin D := Ms() mod 7; exit(Kind(D) + ' ' + Format(D)); end;");
    assert_eq!(text(run(&s, "F")), "Yes/No 1");
}

#[test]
fn bigint_div_stays_a_bigint_in_a_variant() {
    // PBinaryUnaryExpressions.al:141, :180: `Big div 2` is a BigInteger.
    let s = src("procedure F(): Text begin exit(Kind(Ms() div 2)); end;");
    assert_eq!(text(run(&s, "F")), "No/Yes");
}

#[test]
fn decimal_mod_into_integer_is_an_integer() {
    let s = src(
        "procedure F(): Text var D: Integer; Dec: Decimal; begin Dec := 9; D := Dec mod 4; exit(Kind(D) + ' ' + Format(D)); end;",
    );
    assert_eq!(text(run(&s, "F")), "Yes/No 1");
}
