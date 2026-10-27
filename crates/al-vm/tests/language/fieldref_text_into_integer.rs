//! `FieldRef.Value := 'abc'` on an Integer field raises `The value "abc" can't be evaluated into type Integer.`
//! Its code is Evaluate:Format (fieldref/TestFieldRefValue.al:202 FieldRef_Value_TextIntoInteger_ErrorText).

const TABLE: &str =
    "table 50711 Row { fields { field(1; Code; Code[20]) { } field(4; Qty; Integer) { } } keys { key(PK; Code) { } } }";

#[test]
fn letters_into_an_integer_fieldref_raise_the_evaluate_text() {
    let src = format!(
        "{TABLE} codeunit 50764 C {{ procedure F(): Text var RecRef: RecordRef; FldRef: FieldRef; begin
            RecRef.Open(Database::Row); FldRef := RecRef.Field(4); asserterror FldRef.Value := 'abc';
            exit(GetLastErrorText()); end;
          procedure G(): Text var RecRef: RecordRef; FldRef: FieldRef; begin
            RecRef.Open(Database::Row); FldRef := RecRef.Field(4); asserterror FldRef.Value := 'abc';
            exit(GetLastErrorCode()); end; }}"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src, "C", "F")),
        "The value \"abc\" can't be evaluated into type Integer."
    );
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "G")), "Evaluate:Format");
}

#[test]
fn the_other_number_types_and_an_empty_text() {
    let src = format!(
        "{TABLE} table 50712 Nums {{ fields {{ field(1; K; Integer) {{ }} field(2; B; BigInteger) {{ }} field(3; D; Decimal) {{ }} }} keys {{ key(PK; K) {{ }} }} }}
        codeunit 50765 C {{ procedure F(): Text var RecRef: RecordRef; s: Text; begin
            RecRef.Open(Database::Nums);
            asserterror RecRef.Field(2).Value := 'abc'; s := GetLastErrorText() + '|';
            asserterror RecRef.Field(3).Value := 'abc'; s += GetLastErrorText() + '|';
            asserterror RecRef.Field(1).Value := ''; s += GetLastErrorText();
            exit(s); end; }}"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src, "C", "F")),
        "The value \"abc\" can't be evaluated into type BigInteger.|The value \"abc\" can't be evaluated into type Decimal.|The value \"\" can't be evaluated into type Integer."
    );
}
