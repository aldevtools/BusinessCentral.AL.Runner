//! Evaluate(FieldRef, Text) into a Guid field as a statement (the result unused), al-runner issue 6924. Pinned by
//! recordref-fieldref/N582EvalFieldRefUnused.al:218-249 (N582_Evaluate_FieldRef_Guid_Statement); the other field types of
//! :251-332 are checked below.

const GUID_ERR: &str = "Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.|InvalidGuidFormat";

fn eval_statement(input: &str) -> String {
    eval_field(2, input)
}

fn eval_field(no: i32, input: &str) -> String {
    let src = format!(
        "table 50101 T {{ fields {{ field(1; Id; Integer) {{ }} field(2; G; Guid) {{ }} field(3; Big; BigInteger) {{ }} field(4; Tm; Time) {{ }} field(5; Dtm; DateTime) {{ }} field(6; DF; DateFormula) {{ }} field(7; Dur; Duration) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
         codeunit 50100 C {{ [TryFunction] procedure TryEval(F: FieldRef; T: Text) begin Evaluate(F, T); end;
         procedure F(): Text var R: Record T; RR: RecordRef; Fl: FieldRef;
         begin R.G := '{{11111111-2222-3333-4444-555555555555}}'; RR.GetTable(R); Fl := RR.Field({no});
           if TryEval(Fl, '{input}') then exit('no error|' + Format(Fl.Value));
           exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(Fl.Value)); end; }}"
    );
    match al_testkit::run_in(&src, "C", "F") {
        Ok(al_builtins::value::Value::Text(t) | al_builtins::value::Value::Code(t)) => t.to_string(),
        Err(al_builtins::native::Signal::Unsupported(w)) => format!("unsupported:{w}"),
        other => format!("{other:?}"),
    }
}

#[test]
fn a_guid_field_takes_every_pinned_spelling_as_a_statement() {
    let want = "no error|{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}";
    for t in [
        "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}",
        "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        "AAAAAAAABBBBCCCCDDDDEEEEEEEEEEEE",
        "(AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE)",
        " {AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE} ",
    ] {
        assert_eq!(eval_statement(t), want, "{t:?}");
    }
    assert_eq!(
        eval_statement("{00000000-0000-0000-0000-000000000000}"),
        "no error|{00000000-0000-0000-0000-000000000000}"
    );
}

#[test]
fn a_bad_or_empty_text_raises_the_guid_error_and_keeps_the_field() {
    let want = format!("{GUID_ERR}|{{11111111-2222-3333-4444-555555555555}}");
    assert_eq!(eval_statement("abc"), want);
    assert_eq!(eval_statement(""), want);
}

#[test]
fn the_other_pinned_field_types_raise_their_error_as_a_statement() {
    // :309-332 (BigInteger, Time, DateTime, DateFormula, Duration with `abc`; BigInteger and DateFormula with a good text).
    assert_eq!(eval_field(3, "12345678901"), "no error|12345678901");
    assert_eq!(
        eval_field(3, "abc"),
        "The value \"abc\" can't be evaluated into type BigInteger.|Evaluate|0"
    );
    assert_eq!(
        eval_field(4, "abc"),
        "The value \"abc\" can't be evaluated into type Time.|Evaluate|"
    );
    assert_eq!(
        eval_field(5, "abc"),
        "The value \"abc\" can't be evaluated into type DateTime.|Evaluate|"
    );
    assert_eq!(
        eval_field(7, "abc"),
        "The value \"abc\" can't be evaluated into type Duration.|Evaluate|"
    );
}

#[test]
fn a_date_formula_field_takes_a_good_text_as_a_statement() {
    // :316-326 `dateformula ok`. Its `abc` error (text `ABC`) is pinned there for a FieldRef but not built: the formula rule
    // that upper-cases letters excludes `C`.
    assert_eq!(eval_field(6, "<2W>"), "no error|2W");
}
