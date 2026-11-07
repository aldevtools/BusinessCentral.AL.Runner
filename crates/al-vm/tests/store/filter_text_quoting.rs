//! `GetFilter` after `SetFilter(F, '%1', Value)` with a text that holds filter characters (record/filters/W4Filters.al
//! SetFilterPercent1_GreaterThanAndDoubleQuote and _WildcardTogetherWithOperators): a value with `| & < > = ( )` or `..` is
//! quoted, also when it holds `*` or `?`; a double quote stays as it is; a lone `*` shows as `='*'`.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50996 "Fq Row" { fields { field(1; Id; Integer) { } field(2; Txt; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50996 C
{
    local procedure Show(Value: Text): Text
    var R: Record "Fq Row";
    begin
        R.SetFilter(Txt, '%1', Value);
        exit(R.GetFilter(Txt));
    end;

    procedure Quotes(): Text begin exit(Show('a>b') + '#' + Show('>b') + '#' + Show('a>')); end;
    procedure DoubleQuote(): Text begin exit(Show('x"y') + '#' + Show('"y') + '#' + Show('"') + '#' + Show('*"*')); end;
    procedure Wild(): Text
    begin
        exit(Show('a*&b') + '#' + Show('a*|b') + '#' + Show('(a*)') + '#' + Show('a*<b') + '#' + Show('a*=b') + '#' +
             Show('a*..b') + '#' + Show('a?..b') + '#' + Show('*') + '#' + Show('*>*'));
    end;
}
"#;

#[test]
fn a_value_with_operators_is_quoted() {
    assert_eq!(text(run_in(SRC, "C", "Quotes")), "'a>b'#'>b'#'a>'");
    assert_eq!(
        text(run_in(SRC, "C", "Wild")),
        "'a*&b'#'a*|b'#'(a*)'#'a*<b'#'a*=b'#'a*..b'#'a?..b'#='*'#'*>*'"
    );
}

#[test]
fn a_double_quote_stays_as_it_is() {
    assert_eq!(text(run_in(SRC, "C", "DoubleQuote")), "x\"y#\"y#\"#*\"*");
}
