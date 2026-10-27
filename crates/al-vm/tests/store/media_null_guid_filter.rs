//! SetFilter on a Media field with the null Guid as text (al-runner issue 5991, the reporter's repro on real BC; registered
//! assumption `media-filter-null-guid`): an empty field equals the null Guid. Another Guid text stays refused.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;

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
        procedure OtherGuid(): Text begin exit(Format(Count('<>11111111-0000-0000-0000-000000000000'))); end; }";

#[test]
fn an_empty_media_field_equals_the_null_guid() {
    clear_hits();
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "NotNull")), "0");
    assert!(take_hits().contains(&Assumption::MediaFilterNullGuid));
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "IsNull")), "1");
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "FindSet")), "No");
}

#[test]
fn another_guid_text_stays_refused() {
    assert!(matches!(
        al_testkit::run_in(SRC, "C", "OtherGuid"),
        Err(Signal::Unsupported(_))
    ));
}
