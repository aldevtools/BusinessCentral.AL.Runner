//! Behaviors corpus bumps 31 and 32 pin (branch lane-settled-31-32). Each test cites its case.

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn out(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(t) | Value::Code(t)) => t.to_string(),
        Ok(Value::Integer(i)) => i.to_string(),
        Ok(Value::Boolean(b)) => b.to_string(),
        Err(Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

fn run(vars: &str, body: &str, helpers: &str) -> String {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end; {helpers} }}");
    out(al_testkit::run_in(&src, "C", "F"))
}

const CASE_HELPERS: &str = "
    local procedure CodeVarLabel(Value: Code[10]; Lbl: Text): Text
    begin
        case Value of
            Lbl: exit('label');
            else exit('else');
        end;
    end;
    local procedure CodeLiteralRange(Value: Code[10]): Text
    begin
        case Value of
            'abc' .. 'abe': exit('in abc..abe');
            'abc' .. 'klm': exit('in abc..klm');
            else exit('else');
        end;
    end;
    local procedure CodeVarRange(Value: Code[10]; Lo: Text; Hi: Text): Text
    begin
        case Value of
            Lo .. Hi: exit('in');
            else exit('else');
        end;
    end;";

// types/TestCaseContracts.al:509-545 (PYC_Case_CodeSelector_VariableAndRangeLabels): with a Code selector, a Text variable label
// and the bounds of a range label (literals or Text variables) are converted to the selector's Code; a label longer than the Code
// raises StringLengthExceeded.
#[test]
fn case_with_a_code_selector_converts_text_variable_and_range_labels() {
    let r = |vars: &str, body: &str| run(vars, body, CASE_HELPERS);
    assert_eq!(r("C: Code[10];", "C := 'ABC'; exit(CodeVarLabel(C, 'abc'));"), "label");
    assert_eq!(r("C: Code[10];", "C := 'ABC'; exit(CodeVarLabel(C, 'ABC '));"), "label");
    assert_eq!(
        r("C: Code[10];", "C := 'ABC'; exit(CodeVarLabel(C, 'ABCDEFGHIJKL'));"),
        "error:The length of the string is 12, but it must be less than or equal to 10 characters. Value: ABCDEFGHIJKL|StringLengthExceeded"
    );
    assert_eq!(r("C: Code[10];", "C := 'K'; exit(CodeLiteralRange(C));"), "in abc..klm");
    assert_eq!(
        r("C: Code[10];", "C := 'ABD'; exit(CodeLiteralRange(C));"),
        "in abc..abe"
    );
    assert_eq!(r("C: Code[10];", "C := 'K'; exit(CodeVarRange(C, 'a', 'z'));"), "in");
    assert_eq!(
        r("C: Code[10];", "C := 'ABD'; exit(CodeVarRange(C, 'abc', 'abe'));"),
        "in"
    );
}

const LANG_HELPERS: &str = "
    local procedure TakeCode(C: Code[10]): Text
    begin
        exit(C);
    end;";

// text/text-methods/CaseConv.Codeunit.al:280-310 (PYC_CodeAssign_Turkish): under Turkish a text assigned or passed to a Code
// upper-cases `i` to the dotted `İ`; a compare with a literal stays plain.
#[test]
fn a_code_assignment_under_turkish_maps_i_to_the_dotted_capital() {
    let r = |body: &str| run("C: Code[10]; T: Text; S: Integer;", body, LANG_HELPERS);
    assert_eq!(
        r("S := GlobalLanguage(); GlobalLanguage(1055); C := 'iı'; GlobalLanguage(S); exit(C);"),
        "İI"
    );
    assert_eq!(
        r("S := GlobalLanguage(); GlobalLanguage(1055); T := 'istanbul'; C := T; GlobalLanguage(S); exit(C);"),
        "İSTANBUL"
    );
    assert_eq!(
        r("S := GlobalLanguage(); GlobalLanguage(1055); T := TakeCode('iz'); GlobalLanguage(S); exit(T);"),
        "İZ"
    );
    assert_eq!(r("GlobalLanguage(1033); C := 'iz'; exit(C);"), "IZ");
    // Evaluate into a Code under Turkish is not pinned.
    assert!(r("GlobalLanguage(1055); Evaluate(C, 'iz'); exit(C);").starts_with("refused:"));
}

// CaseConv.Codeunit.al:218-276 (PYC_CaseConv_OtherLanguages): Azerbaijani maps I and i as Turkish does; German and Lithuanian as the default.
#[test]
fn uppercase_and_lowercase_under_azerbaijani_follow_the_turkish_rule() {
    let r = |body: &str| run("", body, "");
    assert_eq!(r("GlobalLanguage(1068); exit(LowerCase('I') + UpperCase('i'));"), "ıİ");
    assert_eq!(r("GlobalLanguage(1031); exit(LowerCase('I') + UpperCase('i'));"), "iI");
    assert_eq!(r("GlobalLanguage(1063); exit(LowerCase('I') + UpperCase('i'));"), "iI");
}

// recordref-fieldref/fieldref/SystemClearArraySecret.al:69-90, errors/errorinfo/EnTSecret.al:15-26: Clear of a
// SecretText empties it; a copy taken before keeps its content.
#[test]
fn clear_of_a_secret_text_empties_it_and_leaves_a_copy() {
    let r = |body: &str| run("S: SecretText; O: SecretText;", body, "");
    assert_eq!(
        r(
            "S := SecretStrSubstNo('abc'); O := S; Clear(S); exit(Format(S.IsEmpty()) + Format(O.IsEmpty()) + O.Unwrap());"
        ),
        "YesNoabc"
    );
}
