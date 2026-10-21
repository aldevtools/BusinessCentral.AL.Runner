//! Evaluate of a list of numbers (AL Runner issue 5579), pinned by types/lt-bl-evaluate-lists/EvaluateLists.al.

fn text(vars: &str, body: &str) -> String {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end; }}");
    match al_testkit::run_in(&src, "C", "F") {
        Ok(al_builtins::value::Value::Text(t) | al_builtins::value::Value::Code(t)) => t.to_string(),
        Err(al_builtins::native::Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        Err(al_builtins::native::Signal::Unsupported(w)) => format!("unsupported:{w}"),
        other => format!("{other:?}"),
    }
}

const INPUTS: [&str; 14] = [
    "1;5;10", "1;5", "1;", ";1", ";", "1|5|10", "1:5", "1/5", "1..10", "1 5 10", "1-5", "1&5", "1;5.5", " 1 ; 5 ",
];

/// EvaluateLists.al:33 (Repro_SemicolonList_Decimal): the result used in an if is false and the variable stays 0.
#[test]
fn a_semicolon_list_is_no_decimal() {
    assert_eq!(
        text(
            "N: Decimal;",
            "if Evaluate(N, '1;5;10') then exit('true ' + Format(N, 0, 9)); exit('false ' + Format(N, 0, 9));"
        ),
        "false 0"
    );
}

/// EvaluateLists.al:48-142 (Separators_Decimal, _Integer, _BigInteger): every list is rejected and the variable keeps -1.
#[test]
fn lists_of_numbers_are_rejected_for_every_number_type() {
    for ty in ["Decimal", "Integer", "BigInteger"] {
        for t in INPUTS {
            let got = text(
                &format!("V: {ty}; Ok: Boolean;"),
                &format!("V := -1; Ok := Evaluate(V, '{t}'); exit(Format(Ok) + ' ' + Format(V, 0, 9));"),
            );
            assert_eq!(got, "No -1", "{ty} [{t}]");
        }
    }
}

/// EvaluateLists.al:145-172 (ResultNotUsed_Errors): the error text and code when the result is not used.
#[test]
fn a_list_raises_the_evaluate_error_when_the_result_is_not_used() {
    assert_eq!(
        text(
            "D: Decimal; I: Integer; B: BigInteger; R: Text;",
            "D := -1; asserterror Evaluate(D, '1;5;10'); R := GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(D, 0, 9);
             I := -1; asserterror Evaluate(I, '1;5;10'); R += '#' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(I, 0, 9);
             B := -1; asserterror Evaluate(B, '1|5'); exit(R + '#' + GetLastErrorText() + '|' + GetLastErrorCode());"
        ),
        "The value \"1;5;10\" can't be evaluated into type Decimal.|Evaluate|-1#The value \"1;5;10\" can't be evaluated into type Integer.|Evaluate|-1#The value \"1|5\" can't be evaluated into type BigInteger.|Evaluate"
    );
}
