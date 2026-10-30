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

// Corpus master (corpus PR 886) types/lt-evaluate-format-lasterror-setview/EvaluateDateFormat9Space.al:49-81, :90-120: with
// format 9, white space alone (TAB, blanks, CR, LF, VT, FF, no-break space) reads as the empty date, white space around a date is
// dropped, and a TAB inside the date rejects it with the Date format error.
fn eval9(chars: &str) -> String {
    text(
        "D: Date; T: Text; C: Text[1]; Ok: Boolean; R: Text;",
        &format!(
            "{chars} D := DMY2Date(1, 1, 2000); Ok := Evaluate(D, T, 9); R := Format(Ok) + '|' + Format(D, 0, 9);
             D := DMY2Date(1, 1, 2000); if TryEval9(D, T) then R += ' ok|' + Format(D, 0, 9) else R += ' ' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(D, 0, 9);
             exit(R);"
        ),
    )
}

/// AL that sets `T` to the given parts: a number is that character, any other text is appended as written.
fn build(parts: &[&str]) -> String {
    let mut s = String::from("T := '';");
    for p in parts {
        match p.parse::<u32>() {
            Ok(n) => s.push_str(&format!(" C[1] := {n}; T += C;")),
            Err(_) => s.push_str(&format!(" T += '{p}';")),
        }
    }
    s
}

#[test]
fn format_9_reads_white_space_alone_as_the_empty_date_and_drops_it_around_a_date() {
    for parts in [&["9"][..], &["9", "9"], &[" "], &["  "], &["13"], &["10"], &["13", "10"], &["11"], &["12"], &["160"], &[]] {
        assert_eq!(eval9(&build(parts)), "Yes| ok|", "{parts:?}");
    }
    for parts in [
        &["9", "2026-10-16"][..],
        &["2026-10-16", "9"],
        &[" 2026-10-16"],
        &["2026-10-16 "],
        &["2026-10-16", "13", "10"],
        &["10", "2026-10-16"],
        &["2026-10-16", "160"],
    ] {
        assert_eq!(eval9(&build(parts)), "Yes|2026-10-16 ok|2026-10-16", "{parts:?}");
    }
    assert_eq!(
        eval9(&build(&["2026-10", "9", "-16"])),
        "No|2000-01-01 The format of the Date '2026-10\t-16' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'.|Evaluate|2000-01-01"
    );
}
