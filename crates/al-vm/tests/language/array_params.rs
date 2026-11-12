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

// A one-character literal bounds a Char loop variable as that Char.
#[test]
fn a_one_character_literal_is_a_char_for_loop_bound() {
    let src = r#"
codeunit 69862 FC
{
    procedure F(): Text
    var c: Char; s: Text;
    begin
        for c := '0' to '4' do s += Format(c);
        exit(s);
    end;
}
"#;
    let v = al_testkit::run_in(src, "FC", "F");
    assert!(
        matches!(&v, Ok(al_builtins::value::Value::Text(t)) if &**t == "01234"),
        "{v:?}"
    );
}
