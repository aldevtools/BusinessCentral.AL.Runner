//! `Duration mod Integer` formats as a plain whole number: `3600000 mod 1000000` is '600000', while `div` formats as a
//! Decimal ('1,800,000') (types/date-time/Arithmetic.al:Duration_Arithmetic, :373, :386, :393).

use al_testkit::text;

const SRC: &str = r#"
codeunit 69711 C
{
    procedure F(): Text
    var A: Duration;
    begin
        A := 3600000;
        exit(Format(A mod 1000000) + ';' + Format(A div 2));
    end;
}
"#;

#[test]
fn duration_mod_has_no_thousands_separator() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "600000;1,800,000");
}
