//! `FilterGroup` set to a negative number other than -1 (record/lt-filters-keys-finds/P68F_FilterGroup.al
//! RecordRef_Setter_Negative_And_Large, and the Record twin): the group is stored and read back, as a large positive one
//! is. What a filter set in such a group does is not pinned, so the setters of a filter refuse.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50990 "Fg Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50990 C
{
    procedure ReadBack(): Text
    var Row: Record "Fg Row"; RR: RecordRef;
    begin
        Row.FilterGroup := -100; RR.Open(Database::"Fg Row"); RR.FilterGroup := 3; RR.FilterGroup := -100;
        exit(Format(Row.FilterGroup) + '|' + Format(RR.FilterGroup) + '|' + Format(RR.HasFilter()) + '|' + Format(RR.Count()));
    end;

    procedure SetRangeInGroup()
    var Row: Record "Fg Row";
    begin
        Row.FilterGroup := -100;
        Row.SetRange(Id, 1);
    end;
}
"#;

#[test]
fn a_negative_filter_group_other_than_minus_one_is_stored() {
    assert_eq!(text(run_in(SRC, "C", "ReadBack")), "-100|-100|No|0");
}

#[test]
fn a_filter_set_in_such_a_group_is_refused() {
    let r = run_in(SRC, "C", "SetRangeInGroup");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}
