//! SetFilter on a Media field with a Guid as text (al-runner issue 5991: media/lt-so-media-filter/SoMediaFilter.al
//! SO_MediaFilter_NullGuidForms, SO_MediaFilter_MediaIdOfFilledRow): an empty field equals the null Guid, another Guid matches no
//! empty field.

const SRC: &str = "table 50715 MediaRow { fields { field(1; EntryNo; Integer) { AutoIncrement = true; } field(2; MediaValue; Media) { } field(3; Pics; MediaSet) { } }
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
            exit(GetLastErrorText() + '|' + GetLastErrorCode()); end;
        procedure QuotedEmpty(): Text var R: Record MediaRow; begin asserterror R.SetFilter(MediaValue, '''''');
            exit(GetLastErrorText() + '|' + GetLastErrorCode()); end;
        procedure NotQuotedEmpty(): Text var R: Record MediaRow; begin asserterror R.SetFilter(MediaValue, '<>''''');
            exit(GetLastErrorText() + '|' + GetLastErrorCode()); end;
        local procedure SetCount(Filter: Text): Text var R: Record MediaRow; begin
            R.DeleteAll(); R.Init(); R.EntryNo := 1; R.Insert(); R.Init(); R.EntryNo := 2; R.Insert(); R.SetFilter(Pics, Filter);
            exit(R.GetFilter(Pics) + '|' + Format(R.Count())); end;
        procedure SetNull(): Text begin exit(SetCount('00000000-0000-0000-0000-000000000000')); end;
        procedure SetNotNull(): Text begin exit(SetCount('<>00000000-0000-0000-0000-000000000000')); end;
        procedure SetOtherGuid(): Text begin exit(SetCount('11111111-0000-0000-0000-000000000000')); end;
        procedure SetInvalid(): Text begin exit(SetCount('abc')); end; }";

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

#[test]
fn a_quoted_empty_text_raises_the_guid_format_error() {
    // SoMediaFilter.al:143-147 (SO_MediaFilter_EmptyAndQuoted `quoted empty`, `<> quoted empty`).
    let tail = " is not valid for the MediaValue field on the MediaRow table. Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.|Filter:CSide";
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "QuotedEmpty")),
        format!("The filter \"''\"{tail}")
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "NotQuotedEmpty")),
        format!("The filter \"<>''\"{tail}")
    );
}

#[test]
fn the_null_guid_on_a_media_set_field_matches_the_empty_sets() {
    // SoMediaFilter.al:246-250 (SO_MediaSetFilter_NullGuidForms `null`, `<>null`).
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "SetNull")),
        "{00000000-0000-0000-0000-000000000000}|2"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "SetNotNull")),
        "<>{00000000-0000-0000-0000-000000000000}|0"
    );
}

#[test]
fn other_media_set_filter_texts_stay_refused() {
    // Only the null Guid is pinned on a MediaSet field (SoMediaFilter.al:241-253).
    for f in ["SetOtherGuid", "SetInvalid"] {
        assert!(
            matches!(
                al_testkit::run_in(SRC, "C", f),
                Err(al_builtins::native::Signal::Unsupported(_))
            ),
            "{f}"
        );
    }
}
