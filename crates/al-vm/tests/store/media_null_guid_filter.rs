//! SetFilter on a Media field with a Guid as text (al-runner issue 5991: media/lt-so-media-filter/SoMediaFilter.al
//! SO_MediaFilter_NullGuidForms, SO_MediaFilter_MediaIdOfFilledRow): an empty field equals the null Guid, another Guid matches no
//! empty field.

const SRC: &str = "table 50715 MediaRow { fields { field(1; EntryNo; Integer) { AutoIncrement = true; } field(2; MediaValue; Media) { } }
        keys { key(PK; EntryNo) { } } }
    codeunit 50767 C {
        local procedure Count(Filter: Text): Integer var R: Record MediaRow; RecRef: RecordRef; F: FieldRef; begin
            R.DeleteAll(); R.Init(); R.Insert(); RecRef.Open(Database::MediaRow);
            F := RecRef.Field(R.FieldNo(MediaValue)); F.SetFilter(Filter); exit(RecRef.Count()); end;
        procedure NotNull(): Text begin exit(Format(Count('<>00000000-0000-0000-0000-000000000000'))); end;
        procedure IsNull(): Text begin exit(Format(Count('{00000000-0000-0000-0000-000000000000}'))); end;
        procedure FindSet(): Text var R: Record MediaRow; RecRef: RecordRef; F: FieldRef; begin
            R.DeleteAll(); R.Init(); R.Insert(); RecRef.Open(Database::MediaRow);
            F := RecRef.Field(R.FieldNo(MediaValue)); F.SetFilter('<>00000000-0000-0000-0000-000000000000');
            exit(Format(RecRef.FindSet())); end;
        procedure OtherGuid(): Text begin exit(Format(Count('<>11111111-0000-0000-0000-000000000000'))); end;
        procedure Invalid(): Text var R: Record MediaRow; begin asserterror R.SetFilter(MediaValue, 'abc');
            exit(GetLastErrorText() + '|' + GetLastErrorCode()); end; }";

#[test]
fn an_empty_media_field_equals_the_null_guid() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "NotNull")), "0");
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "IsNull")), "1");
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "FindSet")), "No");
}

#[test]
fn another_guid_text_does_not_match_an_empty_field() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "OtherGuid")), "1");
}

#[test]
fn a_text_that_is_no_guid_raises_the_guid_format_error() {
    // SoMediaFilter.al SO_MediaFilter_InvalidText `abc`.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Invalid")),
        "The filter \"abc\" is not valid for the MediaValue field on the MediaRow table. Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.|Filter:CSide"
    );
}
