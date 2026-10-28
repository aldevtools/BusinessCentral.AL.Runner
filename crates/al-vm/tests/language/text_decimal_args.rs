//! A Decimal where PadStr, CopyStr, DelStr, InsStr and SelectStr take an Integer: a whole value works, a fraction raises
//! ConversionCast (corpus branch al-runtime-rs-staging-pub-tg, text/lt-text-functions/PadStr.Codeunit.al:122-235,
//! CopyStr.Codeunit.al, DelStr.Codeunit.al, InsStr.Codeunit.al, SelectStr.Codeunit.al).

use al_testkit::text;

fn run(decls: &str, body: &str) -> String {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var {decls} R: Text; begin {body} end;
          [TryFunction] local procedure T(D: Decimal; var R: Text) begin R := '[' + PadStr('ab', D, 'x') + ']'; end;
          local procedure Try(D: Decimal): Text var R: Text; begin ClearLastError(); if T(D, R) then exit(R); exit('ERR ' + GetLastErrorCode() + ': ' + GetLastErrorText()); end; }}"
    );
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

const ERR34: &str = "ERR ConversionCast: Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 3.4 to System.Int32.";

#[test]
fn padstr_takes_a_whole_decimal_and_raises_on_a_fraction() {
    // PadStr.Codeunit.al PadStrDecimalFill1: p3_3, p3_3.0 `[abx]`, p3_3.4 and p3_-1.5 ConversionCast.
    assert_eq!(
        run("", "exit(Try(3) + '|' + Try(3.0) + '|' + Try(3.4))"),
        format!("[abx]|[abx]|{ERR34}")
    );
    assert_eq!(
        run("", "exit(Try(-1.5))"),
        "ERR ConversionCast: Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value -1.5 to System.Int32."
    );
}

#[test]
fn copystr_delstr_insstr_selectstr_take_a_whole_decimal() {
    // CopyStr c2_2_3 `[bcd]`, DelStr d2_2_3 `[aef]`, InsStr i_2 `[aXYbcdef]`, SelectStr s_2 `[bb]`.
    assert_eq!(
        run(
            "P: Decimal; L: Decimal;",
            "P := 2; L := 3; exit(CopyStr('abcdef', P, L) + '|' + DelStr('abcdef', P, L) + '|' + InsStr('abcdef', 'XY', P) + '|' + SelectStr(P, 'aa,bb,cc') + '|' + CopyStr('abcdef', L))"
        ),
        "bcd|aef|aXYbcdef|bb|cdef"
    );
}

#[test]
fn copystr_with_two_fractions_names_the_position() {
    // CopyStr c2_2.5_3.4: the position converts first.
    assert_eq!(
        run(
            "P: Decimal; L: Decimal;",
            "P := 2.5; L := 3.4; ClearLastError(); asserterror R := CopyStr('abcdef', P, L); exit(GetLastErrorCode() + ': ' + GetLastErrorText())"
        ),
        "ConversionCast: Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 2.5 to System.Int32."
    );
}
