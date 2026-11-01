//! A failing `Evaluate` statement into a Date raises the date format error for every text in English, with and without number 9;
//! Duration (number 0), Char and BigInteger raise their Evaluate errors (types/lt-y49-evaluate-date-blanks/Y49EvaluateDateBlanks.al,
//! corpus master).

const SRC: &str = r#"
codeunit 50774 C {
    procedure Stmt(T: Text; N: Integer)
    var D: Date;
    begin
        if N < 0 then Evaluate(D, T) else Evaluate(D, T, N);
    end;
    procedure Used(T: Text): Text
    var D: Date;
    begin
        D := 20000101D;
        exit(Format(Evaluate(D, T)) + ' ' + Format(D, 0, 9));
    end;
    procedure Dur(N: Integer)
    var X: Duration;
    begin
        Evaluate(X, 'abc', N);
    end;
    procedure Ch()
    var X: Char;
    begin
        Evaluate(X, 'ab');
    end;
    procedure Bi()
    var X: BigInteger;
    begin
        Evaluate(X, 'x');
    end;
}
"#;

fn date_error(t: &str) -> String {
    format!(
        "The format of the Date '{t}' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'."
    )
}

fn stmt(t: &str, n: i32) -> String {
    use al_builtins::value::Value;
    let r = al_testkit::run_with(SRC, "C", "Stmt", vec![Value::Text(t.into()), Value::Integer(n)]);
    al_testkit::error_message(r.map(|(v, _)| v))
}

#[test]
fn a_failing_date_evaluate_statement_raises_the_date_error_for_every_text() {
    for t in [
        "not a date",
        "abc def",
        "a b",
        " x",
        "12 34",
        "2024 03 05",
        "abc,def",
        "x_y",
        "!",
    ] {
        assert_eq!(stmt(t, -1), date_error(t), "{t}");
        assert_eq!(stmt(t, 9), date_error(t), "{t} 9");
    }
    for t in ["5. März", "12:30"] {
        assert_eq!(stmt(t, 9), date_error(t), "{t} 9");
    }
}

#[test]
fn digits_split_by_blanks_are_no_date_with_the_result_used() {
    use al_builtins::value::Value;
    let r = al_testkit::run_with(SRC, "C", "Used", vec![Value::Text("2024 03 05".into())]);
    assert_eq!(al_testkit::text(r.map(|(v, _)| v)), "No 2000-01-01");
    let r = al_testkit::run_with(SRC, "C", "Used", vec![Value::Text("12 34".into())]);
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn duration_char_and_biginteger_statements_raise_their_evaluate_errors() {
    use al_builtins::value::Value;
    let r = al_testkit::run_with(SRC, "C", "Dur", vec![Value::Integer(0)]);
    assert_eq!(
        al_testkit::error_message(r.map(|(v, _)| v)),
        "The value \"abc\" can't be evaluated into type Duration."
    );
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(SRC, "C", "Ch")),
        "The value \"ab\" can't be evaluated into type Char."
    );
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(SRC, "C", "Bi")),
        "The value \"x\" can't be evaluated into type BigInteger."
    );
}
