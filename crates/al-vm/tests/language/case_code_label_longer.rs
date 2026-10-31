//! A `case` label of type Code[n] with a selector of type Code[m], n > m (al-runner issue 6491). Measured on BC by corpus writer
//! Y20 (types/lt-y20-case-code-label/Y20CaseCodeLabel.al:18): the label is converted to the selector's Code, as a Text label is
//! (types/TestCaseContracts.al:441-466). A value that fits compares as it is; a longer one raises StringLengthExceeded when the
//! label is reached, also when a later label would match, and is never cut to the selector's length.

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn out(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Integer(i)) => i.to_string(),
        Err(Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

const SRC: &str = r#"
codeunit 50100 C
{
    local procedure Pick(Selector: Code[10]; Short: Code[20]; Long: Code[20]): Integer
    begin
        case Selector of
            Short:
                exit(1);
            Long:
                exit(2);
        end;
        exit(0);
    end;

    local procedure PickWide(Selector: Code[20]; Short: Code[10]; Long: Code[10]): Integer
    begin
        case Selector of
            Short:
                exit(1);
            Long:
                exit(2);
        end;
        exit(0);
    end;

    procedure Fits(): Integer
    begin
        exit(Pick('ABC', 'ABC', 'XYZ') * 100 + Pick('XYZ', 'ABC', 'XYZ') * 10 + Pick('Q', 'ABC', 'XYZ'));
    end;

    procedure TenChars(): Integer
    begin
        exit(Pick('ABCDEFGHIJ', 'ABC', 'ABCDEFGHIJ'));
    end;

    procedure LongerFirstNoMatch(): Integer
    begin
        exit(Pick('Q', 'ABCDEFGHIJKLMNO', 'XYZ'));
    end;

    procedure LongerFirstSecondMatches(): Integer
    begin
        exit(Pick('XYZ', 'ABCDEFGHIJKLMNO', 'XYZ'));
    end;

    procedure LongerNotReached(): Integer
    begin
        exit(Pick('ABC', 'ABC', 'ABCDEFGHIJKLMNO'));
    end;

    procedure ElevenAgainstPrefix(): Integer
    begin
        exit(Pick('ABCDEFGHIJ', 'ABCDEFGHIJK', 'XYZ'));
    end;

    procedure Wide(): Integer
    begin
        exit(PickWide('ABCDEFGHIJKLMNO', 'ABC', 'XYZ') * 10 + PickWide('XYZ', 'ABC', 'XYZ'));
    end;
}
"#;

fn run(routine: &str) -> String {
    out(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn a_longer_code_label_type_compares_when_its_value_fits_the_selector() {
    assert_eq!(run("Fits"), "120");
    assert_eq!(run("TenChars"), "2");
    assert_eq!(run("LongerNotReached"), "1");
    assert_eq!(run("Wide"), "2");
}

#[test]
fn a_longer_code_label_value_raises_string_length_exceeded_when_it_is_reached() {
    let fifteen = "error:The length of the string is 15, but it must be less than or equal to 10 characters. Value: ABCDEFGHIJKLMNO|StringLengthExceeded";
    assert_eq!(run("LongerFirstNoMatch"), fifteen);
    assert_eq!(run("LongerFirstSecondMatches"), fifteen);
    assert_eq!(
        run("ElevenAgainstPrefix"),
        "error:The length of the string is 11, but it must be less than or equal to 10 characters. Value: ABCDEFGHIJK|StringLengthExceeded"
    );
}
