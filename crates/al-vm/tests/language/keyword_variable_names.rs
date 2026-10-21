//! A variable named by a keyword. The latest AL compiler (30.0.42.60748, 2026-10-21) accepts `If: Integer;` (and most
//! other keywords, see `VAR_NAME_KEYWORDS` in al-syntax) as a variable name; the variable is then used quoted. Using it
//! unquoted (`If := 1;`) is a compile error there (AL0224), and a parse error here.

#[test]
fn a_variable_named_by_a_keyword_is_used_quoted() {
    let src = "codeunit 50100 C { procedure F(): Integer var If: Integer; While: Integer; begin \"If\" := 5; \"While\" := 2; exit(\"If\" * \"While\"); end; }";
    assert_eq!(al_testkit::int(al_testkit::run_in(src, "C", "F")), 10);
}
