//! An array parameter takes the caller's array (by var) or a copy of it (by value); the callee's own local arrays start
//! with default elements. The default of a parameter that is passed in is not built (it would be thrown away at once).

use al_builtins::value::Value;

fn run(body: &str) -> Value {
    let src = format!(
        "codeunit 50100 C {{
           procedure Fill(var A: array[4] of Integer) begin A[2] := 7; end;
           procedure Peek(A: array[4] of Integer): Integer begin A[2] := 99; exit(A[2]); end;
           procedure Zeroes(): Integer var L: array[3] of Integer; begin exit(L[1] + L[3]); end;
           procedure F(): Integer var X: array[4] of Integer; R: Integer; begin {body} end; }}"
    );
    al_testkit::run_in(&src, "C", "F").unwrap_or(Value::Null)
}

#[test]
fn a_var_array_parameter_is_the_callers_array() {
    assert!(matches!(run("Fill(X); exit(X[2]);"), Value::Integer(7)));
}

#[test]
fn a_by_value_array_parameter_is_a_copy() {
    assert!(matches!(
        run("X[2] := 5; R := Peek(X); exit(R * 1000 + X[2]);"),
        Value::Integer(99005)
    ));
}

#[test]
fn a_local_array_starts_with_default_elements() {
    assert!(matches!(run("exit(Zeroes());"), Value::Integer(0)));
    assert!(matches!(run("exit(X[1] + X[4]);"), Value::Integer(0)));
}

// Base Application's ServContractManagement writes `EntryAmount[AmountType::Amount] -= x` with `AmountType` an Option: an Option
// member is an index (187 Tests-SCM-Service tests were refused with "Option member used as an integer").
#[test]
fn an_option_member_is_an_array_index_in_a_compound_assignment() {
    let src = r#"
codeunit 69860 OI
{
    var AmountType: Option ,Amount,UnitPrice,UnitCost;
    procedure F(): Decimal
    var A: array[3] of Decimal;
    begin
        A[AmountType::UnitPrice] += 5;
        A[AmountType::UnitPrice] -= 2;
        A[AmountType::Amount] += 1;
        exit(A[2] * 10 + A[1]);
    end;
}
"#;
    let v = al_testkit::run_in(src, "OI", "F");
    assert!(
        matches!(&v, Ok(al_builtins::value::Value::Decimal(d)) if *d == rust_decimal::Decimal::from(31)),
        "{v:?}"
    );
}

// Base Application's ServItemManagement loops `for x := 2 to ServLine."Qty. to Ship"` with a Decimal bound.
#[test]
fn a_decimal_with_a_whole_value_is_a_for_loop_bound() {
    let src = r#"
codeunit 69861 FD
{
    procedure F(): Integer
    var x: Integer; q: Decimal; n: Integer;
    begin
        q := 3;
        for x := 1 to q do n += 1;
        exit(n);
    end;
}
"#;
    let v = al_testkit::run_in(src, "FD", "F");
    assert!(matches!(&v, Ok(al_builtins::value::Value::Integer(3))), "{v:?}");
}

// The one-character literal bounds of a Char loop variable that language/ForCharBounds.al measured (open corpus PR 1525) run through
// the assumption for-char-literal-bounds; the corpus case is their test. Every other shape keeps main's refusal (hard rule 3):
// a literal next to a Text variable, an expression or an Integer variable, and a global Char loop variable.
#[test]
fn unmeasured_one_character_literal_for_loop_bounds_are_refused() {
    for (decl, head) in [
        ("var c: Char; t: Text; s: Text;", "for c := '0' to t do"),
        ("var c: Char; i: Integer; s: Text;", "for c := '0' to i do"),
        ("var c: Char; s: Text;", "for c := '0' to 'a' + 1 do"),
        ("var b: BigInteger; c: Char; s: Text;", "for b := '0' to '4' do"),
        ("var d: Decimal; c: Char; s: Text;", "for d := '0' to '4' do"),
        // A non-BMP literal is two UTF-16 units in BC (AL0122), and an Integer literal other than the measured `48 to <literal>` and `<literal> to 52` pairings.
        ("var c: Char; s: Text;", "for c := '\u{1F600}' to '\u{1F601}' do"),
        ("var c: Char; s: Text;", "for c := '0' to 70000 do"),
        ("var c: Char; s: Text;", "for c := '0' to 50 do"),
        ("var c: Char; s: Text;", "for c := 49 to 'a' do"),
        ("var c: Char; s: Text;", "for c := 'a' to 65535 do"),
        ("var c: Char; s: Text;", "for c := 0 to 'a' do"),
        ("var c: Char; s: Text;", "for c := '0' downto 1 do"),
        ("var c: Char; s: Text;", "for c := '0' to 48 do"),
        ("var c: Char; s: Text;", "for c := 52 downto '0' do"),
        ("var c: Char; s: Text;", "for c := '4' downto 48 do"),
        ("var c: Char; s: Text;", "for c := 52 to '0' do"),
        ("var c: Char; s: Text;", "for c := 48 downto '0' do"),
        ("var c: Char; s: Text;", "for c := '4' downto 52 do"),
        // A literal next to a local Char variable was measured only with `to` (Char_LiteralStart_CharVariableEnd, Char_CharVariableStart_LiteralEnd).
        ("var c: Char; l: Char; s: Text;", "for c := 'c' downto l do"),
        ("var c: Char; l: Char; s: Text;", "for c := l downto 'a' do"),
    ] {
        let src = format!(
            "codeunit 69862 FC\n{{\n    procedure F(): Text\n    {decl}\n    begin\n        {head} s += Format(c);\n        exit(s);\n    end;\n}}\n"
        );
        let why = match al_testkit::run_in(&src, "FC", "F") {
            Err(al_builtins::native::Signal::Unsupported(why)) => why.into_reason(),
            other => panic!("`{head}`: expected a refusal, got {other:?}"),
        };
        assert!(why.contains("used as an integer"), "`{head}`: {why}");
    }
}

#[test]
fn a_one_character_literal_char_for_loop_bound_runs_through_its_assumption() {
    let src = r#"
codeunit 69863 FE
{
    procedure F(): Text
    var c: Char; s: Text;
    begin
        for c := '0' to '4' do s += Format(c);
        exit(s);
    end;
}
"#;
    let v = al_testkit::run_in(src, "FE", "F");
    assert!(
        matches!(&v, Ok(al_builtins::value::Value::Text(t)) if t.as_str() == "01234"),
        "{v:?}"
    );
}

// A Char loop variable that is a parameter was not measured (the probe's loop variables are local variables).
#[test]
fn a_parameter_as_the_char_for_loop_variable_keeps_main_s_refusal() {
    let src = r#"
codeunit 69864 FP
{
    local procedure Loop(var c: Char): Integer
    var n: Integer;
    begin
        for c := '0' to '4' do n += 1;
        exit(n);
    end;

    procedure F(): Integer
    var c: Char;
    begin
        exit(Loop(c));
    end;
}
"#;
    match al_testkit::run_in(src, "FP", "F") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.into_reason().contains("used as an integer")),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
