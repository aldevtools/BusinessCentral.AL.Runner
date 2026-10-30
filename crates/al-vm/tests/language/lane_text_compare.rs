//! Code/Text compare and text builtins the corpus pinned and the runtime got wrong (branch lane-text-compare, triage lane T15).
//! Each test cites its case.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

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

fn text(vars: &str, body: &str) -> String {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end; {HELPERS} }}");
    out(al_testkit::run_in(&src, "C", "F"))
}

const HELPERS: &str = "
    local procedure CodeCase(Value: Code[10]): Text
    begin
        case Value of
            'abc': exit('lower label');
            'ABC': exit('upper label');
            else exit('else');
        end;
    end;
    local procedure TextCase(Value: Text): Text
    begin
        case Value of
            'ABC': exit('upper label');
            'abc': exit('lower label');
            else exit('else');
        end;
    end;
    [TryFunction]
    local procedure TryLongLabel(Value: Code[10]; var R: Text)
    begin
        case Value of
            'ABCDEFGHIJKL': R := 'long label';
            'XYZ': R := 'xyz';
            else R := 'else';
        end;
    end;";

// types/TestCaseContracts.al:441-466 (BN_Case_CodeSelectorAndLabels): a Code selector matches a text label as a Code (the
// label `'abc'` matches the Code `ABC` first), a Text selector matches case-sensitively, and a label longer than the Code
// raises StringLengthExceeded with the label in upper case.
#[test]
fn case_with_a_code_selector_compares_the_label_as_a_code() {
    assert_eq!(text("C: Code[10];", "C := 'ABC'; exit(CodeCase(C));"), "lower label");
    assert_eq!(text("C: Code[10];", "C := 'abc'; exit(CodeCase(C));"), "lower label");
    assert_eq!(text("T: Text;", "T := 'abc'; exit(TextCase(T));"), "lower label");
    assert_eq!(text("T: Text;", "T := 'ABC'; exit(TextCase(T));"), "upper label");
    assert_eq!(
        text(
            "C: Code[10]; R: Text;",
            "C := 'XYZ'; if TryLongLabel(C, R) then exit(R); exit(GetLastErrorText() + '|' + GetLastErrorCode());"
        ),
        "The length of the string is 12, but it must be less than or equal to 10 characters. Value: ABCDEFGHIJKL|StringLengthExceeded"
    );
}

// types/lt-ax-text-format-dates/AxCodeCompare.al:174-214 (InRange_CodeAndTextMixedCase): a Text value in a range of Text
// variables or Text literals uses the culture order (`K` is inside `a .. z`, `k` inside `A .. Z`); a Text equals a Code by case.
#[test]
fn a_text_range_end_follows_the_culture_order() {
    let vars = "TVal: Text; TLo: Text; THi: Text; CVal: Code[10];";
    assert_eq!(
        text(
            vars,
            "TVal := 'K'; TLo := 'a'; THi := 'z'; exit(Format(TVal in [TLo .. THi]));"
        ),
        "Yes"
    );
    assert_eq!(text(vars, "TVal := 'K'; exit(Format(TVal in ['A' .. 'z']));"), "Yes");
    assert_eq!(text(vars, "TVal := 'K'; exit(Format(TVal in ['a' .. 'Z']));"), "Yes");
    assert_eq!(text(vars, "TVal := 'k'; exit(Format(TVal in ['A' .. 'Z']));"), "Yes");
    assert_eq!(
        text(vars, "TVal := 'k'; CVal := 'K'; exit(Format(TVal in [CVal]));"),
        "No"
    );
    // types/lt-expression-operators-literals/ExpressionBinaryOperators.al:288, :303: a Char literal in a range of Char literals
    // compares by number.
    assert_eq!(text(vars, "exit(Format('K' in ['a' .. 'z']));"), "No");
}

// text/lt-text-functions/CaseConv.Codeunit.al:127-168 (LT_BU_CaseConv_TurkishGreekLanguage): with GlobalLanguage 1055
// LowerCase('I') is the dotless `ı` and UpperCase('i') the dotted `İ`; ToLower and ToUpper do not follow the language.
#[test]
fn lowercase_and_uppercase_follow_turkish() {
    let r = text(
        "T: Text;",
        "GlobalLanguage(1055); T := 'I'; exit(LowerCase('I') + UpperCase('i') + T.ToLower() + 'i'.ToUpper() + LowerCase('İ') + UpperCase('ı') + LowerCase('ΑΣ') + UpperCase('ας') + UpperCase('ß'));",
    );
    assert_eq!(r, "ıİiIiIασΑΣß");
    let r = text("", "GlobalLanguage(1032); exit(LowerCase('I') + UpperCase('i'));");
    assert_eq!(r, "iI");
    let r = text("", "exit(LowerCase('I') + UpperCase('i'));");
    assert_eq!(r, "iI");
}

// misc/lt-system-functions/ErrorMessage.al:590-620 (LT_BU_Message_BackslashAndAppendLine): Message shows a CR LF, from the text
// or from an argument, as one backslash; the error text of Error(Text) without arguments does too (17 characters).
#[test]
fn message_and_error_show_crlf_as_a_backslash() {
    let src = "codeunit 50100 C {
        procedure M()
        var TB: TextBuilder; n: Integer; t: Text;
        begin
            n := 5; t := 'x';
            Message('A: \\' + 'B:%1\\' + 'C:%2', n, t);
            TB.AppendLine('line1'); TB.AppendLine('line2'); TB.Append('line3');
            Message(TB.ToText());
            Message('%1', TB.ToText());
        end;
        procedure E(): Text
        var TB: TextBuilder;
        begin
            TB.AppendLine('line1'); TB.AppendLine('line2'); TB.Append('line3');
            asserterror Error(TB.ToText());
            exit(GetLastErrorText() + '|' + Format(StrLen(GetLastErrorText())) + '|' + Format(StrLen(TB.ToText())));
        end;
    }";
    let mut b = Built::from_source(src);
    b.session.capture_messages();
    b.run_in("C", "M").unwrap();
    assert_eq!(
        b.session.take_messages(),
        vec!["A: \\B:5\\C:x", "line1\\line2\\line3", "line1\\line2\\line3"]
    );
    assert_eq!(out(b.run_in("C", "E")), "line1\\line2\\line3|17|19");
}

// misc/lt-system-functions/ErrorMessage.al:684-718 (PYC_Error_CrLfAndLoneCrLf): a CR LF in an argument of Error shows as one
// backslash, in the text and next to other text; a backslash written in the text stays one backslash.
#[test]
fn error_with_an_argument_that_holds_crlf_shows_a_backslash() {
    let r = text(
        "TB: TextBuilder; R: Text;",
        "TB.AppendLine('a'); TB.Append('b'); asserterror Error('%1', TB.ToText()); R := GetLastErrorText(); \
         asserterror Error('x%1y', TB.ToText()); R += '|' + GetLastErrorText(); \
         asserterror Error('p\\q %1', TB.ToText()); exit(R + '|' + GetLastErrorText());",
    );
    assert_eq!(r, "a\\b|xa\\by|p\\q a\\b");
}

// record/lt-as-records/AsCodeKeySigns.al:13-124: the key order of Code values drops every leading hyphen, and of two that tie
// there the one with fewer hyphens comes first; a filter on such values uses the same order.
#[test]
fn a_code_key_drops_every_leading_hyphen() {
    let src = "table 50101 K { fields { field(1; \"Code\"; Code[20]) { } } keys { key(PK; \"Code\") { } } }
    codeunit 50100 C {
        local procedure Seed()
        var K: Record K;
        begin
            K.DeleteAll();
            Add('-+5'); Add('-A'); Add('+5'); Add('-0'); Add('0'); Add('5'); Add('A'); Add('-5'); Add('+A'); Add('--5');
        end;
        local procedure Add(V: Code[20])
        var K: Record K;
        begin
            K.\"Code\" := V; K.Insert();
        end;
        local procedure List(var K: Record K): Text
        var R: Text;
        begin
            if K.FindSet() then repeat R += K.\"Code\" + ' '; until K.Next() = 0;
            exit(R.TrimEnd());
        end;
        procedure F(): Text
        var K: Record K; R: Text;
        begin
            Seed();
            R := List(K);
            K.Ascending(false); R += '|' + List(K); K.Ascending(true);
            K.SetRange(\"Code\", '-0', 'A'); R += '|' + List(K);
            K.SetFilter(\"Code\", '<%1', '-A'); R += '|' + List(K);
            K.SetFilter(\"Code\", '-*'); R += '|' + List(K);
            exit(R);
        end;
    }";
    assert_eq!(
        out(al_testkit::run_in(src, "C", "F")),
        "+5 -+5 +A 0 -0 5 -5 --5 A -A|-A A --5 -5 5 -0 0 +A -+5 +5|-0 5 -5 --5 A|+5 -+5 +A 0 -0 5 -5 --5 A|-+5 -0 -5 --5 -A"
    );
}
