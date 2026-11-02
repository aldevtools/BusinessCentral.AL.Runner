//! `Evaluate(RecordId, ..)` on a table whose primary key is a Code and a Text field
//! (misc/lt-y68-small-probes/Y68Small.al:65-121 Q5_Evaluate_RecordId_CodeAndText_Forms, corpus master 0d01b0cc).

const SRC: &str = r#"
table 91600 "RCT Key" { fields { field(1; A; Code[20]) { } field(2; B; Text[50]) { } } keys { key(PK; A, B) { } } }
codeunit 91601 C
{
    local procedure Eval(Value: Text): Text
    var
        Rid: RecordId;
    begin
        if not Evaluate(Rid, Value) then
            exit('No');
        exit(Format(Rid));
    end;
    procedure All(): Text
    begin
        exit(Eval('RCT Key: aBc,hELLO World') + '|' + Eval('RCT Key: A1 , Hello') + '|' + Eval('RCT Key:  A1,Hello ') + '|' +
            Eval('RCT Key: A1,') + '|' + Eval('RCT Key: A1,""') + '|' + Eval('RCT Key: ,Hello') + '|' + Eval('RCT Key: ,') + '|' +
            Eval('RCT Key: A1,"x,y"') + '|' + Eval('RCT Key: A1,"say ""hi"""') + '|' + Eval('RCT Key: A"1,say "hi"') + '|' +
            Eval('RCT Key: A1," "') + '|' + Eval('RCT Key: A1,x,y') + '|' + Eval('RCT Key: ABCDEFGHIJKLMNOPQRSTU,Hello') + '|' +
            Eval('RCT Key: A1;Hello'));
    end;
    procedure QuoteAfterBlank(): Text begin exit(Eval('RCT Key: A1, "x"')); end;
}
"#;

#[test]
fn a_code_and_text_key_reads_as_pinned() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "All")),
        [
            "RCT Key: ABC,hELLO World",
            "RCT Key: A1,Hello",
            "RCT Key: A1,Hello",
            "RCT Key: A1",
            "RCT Key: A1,\"\"",
            "RCT Key: \"\",Hello",
            "RCT Key: \"\"",
            "RCT Key: A1,\"x,y\"",
            "RCT Key: A1,say \"hi\"",
            "RCT Key: A\"1,say \"hi\"",
            "RCT Key: A1,\" \"",
            "No",
            "No",
            "RCT Key: A1;HELLO",
        ]
        .join("|")
    );
}

#[test]
fn a_part_that_starts_with_a_quote_after_blanks_is_refused() {
    assert!(matches!(
        al_testkit::run_in(SRC, "C", "QuoteAfterBlank"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}
