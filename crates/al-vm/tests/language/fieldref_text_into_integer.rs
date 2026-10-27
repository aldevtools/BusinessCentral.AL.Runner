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

/// A text of letters into a Boolean, Date, Guid or Option field through FieldRef.Value raises and keeps the field
/// (fieldref/TestFieldRefValue.al:282 FieldRef_Value_TextIntoBooleanDateGuidOption).
#[test]
fn letters_into_boolean_date_guid_and_option() {
    let src = "table 50713 Kinds { fields { field(1; K; Integer) { } field(2; B; Boolean) { } field(3; D; Date) { }
            field(4; G; Guid) { } field(5; O; Option) { OptionMembers = A,B; } } keys { key(PK; K) { } } }
        codeunit 50766 C { procedure F(): Text var RecRef: RecordRef; s: Text; i: Integer; begin
            RecRef.Open(Database::Kinds);
            asserterror RecRef.Field(2).Value := 'abc'; s := GetLastErrorCode() + ':' + GetLastErrorText() + '|';
            asserterror RecRef.Field(2).Value := 'true'; s += GetLastErrorCode() + ':' + GetLastErrorText() + '|';
            asserterror RecRef.Field(3).Value := 'abc'; s += GetLastErrorCode() + '|';
            asserterror RecRef.Field(4).Value := 'abc'; s += GetLastErrorCode() + '|';
            asserterror RecRef.Field(5).Value := 'abc'; s += GetLastErrorCode() + ':' + GetLastErrorText() + '|';
            for i := 2 to 5 do s += Format(RecRef.Field(i).Value, 0, 9) + ',';
            exit(s); end; }";
    assert_eq!(
        al_testkit::text(al_testkit::run_in(src, "C", "F")),
        "Evaluate:Format:The value \"abc\" can't be evaluated into type Boolean.|Evaluate:Format:The value \"true\" can't be evaluated into type Boolean.|Evaluate:Conversion|InvalidGuidFormat|Evaluate:Format:The value \"abc\" can't be evaluated into type Option.|false,,{00000000-0000-0000-0000-000000000000},0,"
    );
}
