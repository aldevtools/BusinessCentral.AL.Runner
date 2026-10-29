fn text(vars: &str, body: &str) -> String {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end;
         [TryFunction] procedure TryEval9(var D: Date; T: Text) begin Evaluate(D, T, 9); end;
         [TryFunction] procedure TryEval0(var D: Date; T: Text) begin Evaluate(D, T, 0); end;
         [TryFunction] procedure TryEval(var D: Date; T: Text) begin Evaluate(D, T); end; }}"
    );
    match al_testkit::run_in(&src, "C", "F") {
        Ok(al_builtins::value::Value::Text(t) | al_builtins::value::Value::Code(t)) => t.to_string(),
        Err(al_builtins::native::Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        Err(al_builtins::native::Signal::Unsupported(w)) => format!("unsupported:{w}"),
        other => format!("{other:?}"),
    }
}

// errors/lt-ms-refusals-130/P130EvaluateDate.al UN_Evaluate_Statement_NotADate_ByTarget (corpus branch
// al-runtime-rs-staging-un, commit d3bcc580): an Evaluate statement of `not-a-date` into a Date raises the
// Date-format error with format 9, 0 and the default; the value is kept.
#[test]
fn an_evaluate_statement_of_letters_into_a_date_raises_the_date_format_error() {
    let want = "The format of the Date 'not-a-date' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'.|Evaluate|01/02/25";
    for call in ["TryEval9", "TryEval0", "TryEval"] {
        let r = text(
            "D: Date;",
            &format!(
                "D := DMY2Date(2, 1, 2025); if {call}(D, 'not-a-date') then exit('no error'); exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(D));"
            ),
        );
        assert_eq!(r, want, "{call}");
    }
}
