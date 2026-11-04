//! Code comparison (text/text-methods/TextCompare.Codeunit.al:CodeComparison, lines 205-246, BC 27.5 and 29): two
//! Code values compare without regard to case, and a Code of digits only is ordered as a number (`'10' < '9'` is No). A
//! Text compares as before (a25 and a26 of the same file). Base Application's `NoIsWithinValidRange` compares the last
//! number used with the ending number of a number series as Codes: with a text order the Cronus series `P-CR+`
//! (109001 to 1010999) read as used up, and 36 purchase correction tests were refused.

use al_testkit::{run_in, text};

fn observe(body: &str) -> String {
    let src = format!("codeunit 50100 C {{ var C5: Code[5]; C10: Code[10]; procedure F(): Text begin {body} end; }}");
    text(run_in(&src, "C", "F"))
}

#[test]
fn code_values_compare_as_the_corpus_pins() {
    let r = observe("C5 := 'abc'; C10 := 'ABC'; exit(Format(C5 = C10));");
    assert_eq!(r, "Yes", "eq");
    let r = observe(
        "C5 := 'abc'; C10 := 'abd'; exit(Format(C5 < C10) + Format(C5 > C10) + Format(C5 <= C10) + Format(C5 >= C10) + Format(C5 = 'ABC') + Format(C5 < 'abd'));",
    );
    assert_eq!(r, "YesNoYesNoYesYes", "lt gt le ge lit litlt");
    let r = observe("C5 := 'A'; C10 := 'a'; exit(Format(C5 = C10) + Format(C5 < C10));");
    assert_eq!(r, "YesNo", "same ltcase");
    let r = observe("C5 := 'B'; C10 := 'a'; exit(Format(C5 < C10));");
    assert_eq!(r, "No", "blt");
    let r = observe("C5 := '1'; C10 := 'A'; exit(Format(C5 < C10));");
    assert_eq!(r, "Yes", "numlt");
    let r = observe("C5 := ''; C10 := 'A'; exit(Format(C5 < C10));");
    assert_eq!(r, "Yes", "emptylt");
    let r = observe("C5 := '10'; C10 := '9'; exit(Format(C5 < C10));");
    assert_eq!(r, "No", "tenlt");
}

#[test]
fn a_code_of_digits_is_ordered_as_a_number_and_a_text_is_not() {
    // The number series line of the Cronus series P-CR+: the first number is below the ending number.
    let src = "codeunit 50100 C { procedure F(Cur: Code[20]; Ending: Code[20]): Boolean begin exit(Cur > Ending); end; procedure G(): Text begin exit(Format(F('109001', '1010999'))); end; }";
    assert_eq!(text(run_in(src, "C", "G")), "No");
    // Two Code variables of different length, as NoIsWithinValidRange gets them.
    let r = observe("C10 := '109001'; C5 := '10'; exit(Format(C10 > C5) + Format(C5 < C10));");
    assert_eq!(r, "YesYes");
    // Text keeps its order: a25 and a26.
    let src = "codeunit 50100 C { procedure F(): Text var A: Text; B: Text; begin A := '10'; B := '9'; exit(Format(A < B)); end; }";
    assert_eq!(text(run_in(src, "C", "F")), "Yes");
}
