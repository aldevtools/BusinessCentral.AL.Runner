//! `FieldRef.TestField(5)` on a Text field holding '5' holds for a BigInteger, a Decimal and an Integer: the number is
//! compared as the text it writes (record/testfield/FieldRefTestFieldMismatchedType.al:206-211 and :234,
//! P189_FldRefMismatch_OnTextField, Big_result Yes).

const TABLE: &str =
    "table 50731 TxtRow { fields { field(1; Id; Integer) { } field(2; Txt; Text[50]) { } } keys { key(PK; Id) { } } }";

fn run(arg: &str) -> String {
    al_testkit::text(run_with(arg, "5", "5"))
}

fn run_with(
    arg: &str,
    field_text: &str,
    value: &str,
) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let src = format!(
        "{TABLE} codeunit 50731 C {{
          [TryFunction] local procedure T(var Fld: FieldRef; V: {arg}) begin Fld.TestField(V); end;
          procedure F(): Text var Rec: Record TxtRow; RecRef: RecordRef; Fld: FieldRef; V: {arg}; begin
            Rec.Txt := '{field_text}'; RecRef.GetTable(Rec); Fld := RecRef.Field(2); V := {value};
            if T(Fld, V) then exit('[]');
            exit('[' + GetLastErrorText() + ']'); end; }}"
    );
    al_testkit::run_in(&src, "C", "F")
}

#[test]
fn a_biginteger_equal_to_the_text_holds() {
    assert_eq!(run("BigInteger"), "[]");
}

#[test]
fn a_decimal_and_an_integer_equal_to_the_text_hold() {
    assert_eq!(run("Decimal"), "[]");
    assert_eq!(run("Integer"), "[]");
}

#[test]
fn true_against_the_text_five_holds_and_other_texts_are_refused() {
    // corpus :219 (Bool_result Yes); no other text is pinned.
    assert_eq!(al_testkit::text(run_with("Boolean", "5", "true")), "[]");
    assert!(matches!(
        run_with("Boolean", "7", "true"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}

#[test]
fn an_enum_against_the_text_is_never_equal_and_shows_its_member() {
    // corpus :229-231 (Knd_result No, Knd|text, Knd|code TestField).
    let src = format!(
        "{TABLE} enum 50731 Kind {{ value(0; One) {{ }} value(1; Two) {{ }} }}
        codeunit 50732 C {{
          [TryFunction] local procedure T(var Fld: FieldRef; V: Enum Kind) begin Fld.TestField(V); end;
          procedure F(): Text var Rec: Record TxtRow; RecRef: RecordRef; Fld: FieldRef; V: Enum Kind; begin
            Rec.Txt := '5'; RecRef.GetTable(Rec); Fld := RecRef.Field(2); V := V::Two;
            if T(Fld, V) then exit('held');
            exit(GetLastErrorText() + '|' + GetLastErrorCode()); end; }}"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src, "C", "F")),
        "Txt must be equal to 'Two'  in TxtRow: Id=0. Current value is '5'.|TestField"
    );
}
