//! `FieldRef.GetEnumValueCaptionFromOrdinalValue(FieldRef.Value())` on an Option field takes the Option value as its ordinal
//! (al-runner issue 5987, the reporter's repro on real BC; registered assumption `fieldref-ordinal-from-option-value`).

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
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
            exit(F.GetEnumValueCaptionFromOrdinalValue(F.Value())); end; }";

#[test]
fn an_option_value_is_taken_as_its_ordinal() {
    clear_hits();
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Caption")), "Option1");
    assert!(take_hits().contains(&Assumption::FieldRefOrdinalFromOptionValue));
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Name")), "Option2");
}

#[test]
fn an_enum_value_stays_refused() {
    assert!(matches!(
        al_testkit::run_in(SRC, "C", "EnumArg"),
        Err(Signal::Unsupported(_))
    ));
}
