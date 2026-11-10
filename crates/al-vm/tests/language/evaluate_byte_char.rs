//! `Evaluate` into a Byte and a Char reads one character as its code; a number of two or more characters is rejected
//! (types/decimal-integer/ScT70055.al:205-249, EvaluateByte_Char; format-evaluate/EvalLastError.al:342-343).

use al_builtins::native::Signal;

fn run(src: &str) -> Result<String, Signal> {
    al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| al_testkit::text(Ok(v)))
}

#[test]
fn byte_and_char_read_one_character_as_its_code() {
    let src = "codeunit 50100 C { procedure F(): Text var B: Byte; C: Char; R: Text; begin \
        B := 1; R := Format(Evaluate(B, '0')) + Format(B); B := 1; R += '|' + Format(Evaluate(B, '256')) + Format(B); \
        B := 1; R += '|' + Format(Evaluate(B, '-1')) + Format(B); C := 'z'; R += '|' + Format(Evaluate(C, 'A')) + Format(C); \
        C := 'z'; R += '|' + Format(Evaluate(C, '')) + Format(C); C := 'z'; R += '|' + Format(Evaluate(C, '70000')) + Format(C); \
        exit(R); end; }";
    assert_eq!(run(src).unwrap(), "Yes0|No\u{1}|No\u{1}|YesA|Noz|Noz");
}

#[test]
fn the_unpinned_byte_and_char_texts_are_refused() {
    // A Char reads `ab` as rejected since the matrix pinned it (format-evaluate/format-matrix/FMEvalChar.al:73 `AB`).
    for (ty, text) in [("Byte", "ab"), ("Byte", ""), ("Byte", "é")] {
        let src = format!(
            "codeunit 50100 C {{ procedure F(): Boolean var V: {ty}; begin exit(Evaluate(V, '{text}')); end; }}"
        );
        match run(&src) {
            Err(Signal::Unsupported(_)) => {}
            other => panic!("{ty} {text}: {other:?}"),
        }
    }
}
