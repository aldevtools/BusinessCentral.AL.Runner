//! An Integer next to a one-character text literal under `+`, `-`, `=` and `>`: the literal is a Char, so the two add, subtract
//! and compare as numbers (types/lt-wc/WcIntegerCharLiteral.al, al-runner issue 6300).

use al_testkit::text;

const SRC: &str = r#"
codeunit 50488 C
{
    procedure Arith(): Text
    var I: Integer; Num: Integer; Out: Text;
    begin
        Num := 1000;
        Out := Format(1000 + '-' + 2) + '|' + Format(Num + '-' + 2);
        I := Num + '-'; Out += '|' + Format(I);
        I := '-' + Num; Out += '|' + Format(I);
        I := Num - 'A'; Out += '|' + Format(I);
        I := 'A' - Num; Out += '|' + Format(I);
        I := Num + '0'; Out += '|' + Format(I);
        exit(Out);
    end;
    procedure Compare(): Text
    var Num: Integer;
    begin
        Num := 45;
        exit(Format(Num = '-') + '|' + Format(Num > '-'));
    end;
}
"#;

/// WcIntegerCharLiteral.al:13-42 WC_Integer_Plus_CharLiteral.
#[test]
fn an_integer_and_a_one_character_literal_add_as_numbers() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Arith")),
        "1047|1047|1045|1045|935|-935|1048"
    );
}

/// WcIntegerCharLiteral.al:44-57 WC_Integer_CharLiteral_Compare.
#[test]
fn an_integer_and_a_one_character_literal_compare_as_numbers() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Compare")), "Yes|No");
}
