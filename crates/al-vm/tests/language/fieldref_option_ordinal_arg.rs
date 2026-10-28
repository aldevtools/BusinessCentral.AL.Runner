//! `FieldRef.GetEnumValueCaptionFromOrdinalValue(FieldRef.Value())` on an Option field takes the Option value as its ordinal
//! (al-runner issue 5987: fieldref/TestFieldRefEnumOrdinalByIndex.al SO_OptionField_ValueAsOrdinalArgument_ReproShape); an ordinal
//! the field has no member for answers the number as its caption and an empty name (SO_OptionField_IntegerOrdinalArguments).

use al_builtins::native::Signal;

const SRC: &str = "table 50713 OptRow { fields { field(1; EntryNo; Integer) { } field(2; OptionValue; Option) { OptionMembers = Option1, Option2; }
        field(3; E; Enum OrdE) { } } keys { key(PK; EntryNo) { } } }
    enum 50714 OrdE { value(0; A) { } value(1; B) { } }
    codeunit 50766 C {
        procedure Caption(): Text var R: Record OptRow; RecRef: RecordRef; F: FieldRef; begin
            R.Init(); RecRef.GetTable(R); F := RecRef.Field(R.FieldNo(OptionValue));
            exit(F.GetEnumValueCaptionFromOrdinalValue(F.Value())); end;
        procedure Name(): Text var R: Record OptRow; RecRef: RecordRef; F: FieldRef; begin
            R.Init(); R.OptionValue := R.OptionValue::Option2; RecRef.GetTable(R); F := RecRef.Field(2);
            exit(F.GetEnumValueNameFromOrdinalValue(F.Value())); end;
        procedure EnumArg(): Text var R: Record OptRow; RecRef: RecordRef; F: FieldRef; begin
            R.Init(); RecRef.GetTable(R); F := RecRef.Field(3);
            exit(F.GetEnumValueCaptionFromOrdinalValue(F.Value())); end;
        procedure Missing(): Text var RecRef: RecordRef; F: FieldRef; begin
            RecRef.Open(Database::OptRow); F := RecRef.Field(2);
            exit(F.GetEnumValueCaptionFromOrdinalValue(-1) + '|' + F.GetEnumValueNameFromOrdinalValue(-1) + '|'
                + F.GetEnumValueCaptionFromOrdinalValue(5) + '|' + F.GetEnumValueNameFromOrdinalValue(5)); end; }";

#[test]
fn an_option_value_is_taken_as_its_ordinal() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Caption")), "Option1");
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Name")), "Option2");
}

#[test]
fn an_enum_value_stays_refused() {
    assert!(matches!(
        al_testkit::run_in(SRC, "C", "EnumArg"),
        Err(Signal::Unsupported(_))
    ));
}

#[test]
fn an_ordinal_without_a_member_answers_the_number_and_no_name() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Missing")), "-1||5|");
}
