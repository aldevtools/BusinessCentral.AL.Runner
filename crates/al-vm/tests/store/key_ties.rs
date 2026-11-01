//! Sorting by a key whose rows tie, and the primary key fields that follow a key
//! (corpus record/lt-setascending-ties/SetAscendingTies.al).

use al_builtins::native::Signal;
use al_testkit::{error_message, text};

const SRC: &str = r#"
table 69990 "KT Row"
{
    fields { field(1; Id; Integer) { } field(2; Grp; Integer) { } }
    keys { key(PK; Id) { } key(ByGrp; Grp) { } key(ByGrpId; Grp, Id) { } }
}
codeunit 69991 C
{
    local procedure Seed()
    var Row: Record "KT Row"; I: Integer;
    begin
        for I := 1 to 6 do begin
            Row.Init(); Row.Id := I; Row.Grp := (I - 1) div 3 + 1; Row.Insert();
        end;
    end;

    local procedure Ids(var Row: Record "KT Row"): Text
    var s: Text;
    begin
        if Row.FindSet() then
            repeat s += Format(Row.Id) + ','; until Row.Next() = 0;
        exit(s);
    end;

    procedure ExplicitPrimaryKey(): Text
    var Row: Record "KT Row";
    begin
        Seed();
        Row.SetCurrentKey(Grp, Id);
        exit(Ids(Row) + '|' + Row.GetView(false));
    end;

    procedure ExplicitPrimaryKeySetId()
    var Row: Record "KT Row";
    begin
        Row.SetCurrentKey(Grp, Id);
        Row.SetAscending(Id, false);
    end;

    procedure GrpDescIdAsc(): Text
    var Row: Record "KT Row";
    begin
        Seed();
        Row.SetCurrentKey(Grp);
        Row.SetAscending(Grp, false);
        Row.SetAscending(Id, true);
        exit(Ids(Row) + '|' + Row.GetView(false));
    end;

    procedure IdAscOnPlainKey()
    var Row: Record "KT Row";
    begin
        Row.SetCurrentKey(Grp);
        Row.SetAscending(Id, true);
    end;

    procedure IdDescAfterGrpDesc()
    var Row: Record "KT Row";
    begin
        Row.SetCurrentKey(Grp);
        Row.SetAscending(Grp, false);
        Row.SetAscending(Id, false);
    end;
}
"#;

// SetAscendingTies.al:173 (`key_grp_id_plain`): the primary key field named after the key is not part of the sorting.
#[test]
fn explicit_primary_key_field_after_the_key_is_dropped() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "ExplicitPrimaryKey")),
        "1,2,3,4,5,6,|VERSION(1) SORTING(Field2)"
    );
    // SetAscendingTies.al:174 (`key_grp_id_id_desc`).
    assert_eq!(
        error_message(al_testkit::run_in(SRC, "C", "ExplicitPrimaryKeySetId")),
        "The specified sorting could not be set on field 'Id' with the key consisting of 'Grp'."
    );
}

// SetAscendingTies.al:155 (`grp_desc_id_asc`): asking the following primary key field for the direction it already has is
// accepted and changes nothing.
#[test]
fn primary_key_field_after_a_descending_key_field_accepts_ascending() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "GrpDescIdAsc")),
        "4,5,6,1,2,3,|VERSION(1) SORTING(Field2) ORDER(1)"
    );
}

// Not pinned: the same request on a key with no descending field, and a descending request after a descending key field.
#[test]
fn unpinned_requests_on_a_following_primary_key_field_are_refused() {
    for routine in ["IdAscOnPlainKey", "IdDescAfterGrpDesc"] {
        let r = al_testkit::run_in(SRC, "C", routine);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
    }
}

const FLOW: &str = r#"
table 69994 "KT Owner"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count("KT Line" where(OwnerId = field(Id))); }
        field(3; All; Integer) { FieldClass = FlowField; CalcFormula = count("KT Line"); }
    }
    keys { key(PK; Id) { } }
}
table 69995 "KT Line" { fields { field(1; Id; Integer) { } field(2; OwnerId; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69996 C
{
    procedure ByRowFlowField(): Text
    var Owner: Record "KT Owner";
    begin
        Owner.SetCurrentKey(Cnt);
        exit(Owner.CurrentKey());
    end;

    procedure ByIdThenRowFlowField(): Text
    var Owner: Record "KT Owner";
    begin
        Owner.SetCurrentKey(Id, Cnt);
        exit(Owner.CurrentKey());
    end;

    procedure ByRowFlowFieldThenClear(): Text
    var Owner: Record "KT Owner";
    begin
        Owner.SetCurrentKey(Cnt);
        Clear(Owner);
        exit(Owner.CurrentKey());
    end;

    procedure ByIdThenRowFlowFieldThenFind()
    var Owner: Record "KT Owner";
    begin
        Owner.SetCurrentKey(Id, Cnt);
        if Owner.FindSet() then;
    end;

    procedure ByConstantFlowField(): Text
    var Owner: Record "KT Owner";
    begin
        Owner.SetCurrentKey(All);
        exit(Owner.CurrentKey());
    end;
}
"#;

// records/W4SortKeys.al:459, :471: a FlowField whose value is the same for every row is accepted and the key stays the primary
// key. A FlowField that depends on the row is reported as the key, alone and after a normal field (record/
// Y8SetCurrentKeyNotStored.al:84-106; SetCurrentKeyFlowFieldOrder.al:109-113). The order of the rows is built only with the
// FlowField first (flowfield_key_order.rs): after a normal field a read refuses.
#[test]
fn set_current_key_on_a_flow_field_that_depends_on_the_row_names_it_and_refuses_reads() {
    assert_eq!(text(al_testkit::run_in(FLOW, "C", "ByConstantFlowField")), "Id");
    assert_eq!(text(al_testkit::run_in(FLOW, "C", "ByRowFlowField")), "Cnt");
    assert_eq!(text(al_testkit::run_in(FLOW, "C", "ByIdThenRowFlowField")), "Id,Cnt");
    assert_eq!(text(al_testkit::run_in(FLOW, "C", "ByRowFlowFieldThenClear")), "Id");
    let r = al_testkit::run_in(FLOW, "C", "ByIdThenRowFlowFieldThenFind");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

const MEDIA: &str = r#"
table 69994 "KT Media Row"
{
    fields { field(1; Id; Integer) { } field(2; Pic; Media) { } field(3; Pics; MediaSet) { } }
    keys { key(PK; Id) { } }
}
codeunit 69995 C
{
    local procedure Seed()
    var Row: Record "KT Media Row"; I: Integer;
    begin
        for I := 1 to 3 do begin
            Row.Init(); Row.Id := I; Row.Insert();
        end;
    end;

    local procedure KeyOfCopy(Row: Record "KT Media Row"): Text
    begin
        exit(Row.CurrentKey());
    end;

    local procedure FindOnCopy(Row: Record "KT Media Row"): Text
    begin
        Row.FindFirst();
        exit(Format(Row.Id));
    end;

    procedure DirectMedia(): Text
    var Row: Record "KT Media Row";
    begin
        Seed();
        Row.SetCurrentKey(Pic);
        exit(Row.CurrentKey());
    end;

    procedure DirectMediaSet(): Text
    var Row: Record "KT Media Row";
    begin
        Seed();
        Row.SetCurrentKey(Pics);
        exit(Row.CurrentKey());
    end;

    procedure ThroughCopy(): Text
    var Row: Record "KT Media Row";
    begin
        Seed();
        Row.SetCurrentKey(Pic);
        exit(KeyOfCopy(Row));
    end;

    procedure ResetClears(): Text
    var Row: Record "KT Media Row";
    begin
        Seed();
        Row.SetCurrentKey(Pic);
        Row.Reset();
        Row.FindLast();
        exit(Row.CurrentKey() + Format(Row.Id));
    end;

    procedure FindsInMediaKey(): Text
    var Row: Record "KT Media Row";
    begin
        Seed();
        Row.SetCurrentKey(Pic);
        Row.FindFirst();
        exit('');
    end;

    procedure ViewOfMediaKey(): Text
    var Row: Record "KT Media Row";
    begin
        Row.SetCurrentKey(Pic);
        exit(Row.GetView());
    end;

    procedure FindsOnCopy(): Text
    var Row: Record "KT Media Row";
    begin
        Seed();
        Row.SetCurrentKey(Pic);
        exit(FindOnCopy(Row));
    end;
}
"#;

// SetCurrentKey on a Media or MediaSet field that no key holds: the record itself answers the field as its key
// (record/lt-ar-records/ArRecordProbes.al:198-229 SetCurrentKey_Media `Pic`, SetCurrentKey_MediaSet `Pics`), a by-value copy of it
// answers the primary key (records/W4SortKeys.al:440-471 `media` `ok:[Id]`). The order of the rows, the view and the rest of
// what such a record answers are not pinned: refused.
#[test]
fn set_current_key_on_a_media_field_answers_the_field_and_its_copy_the_primary_key() {
    assert_eq!(text(al_testkit::run_in(MEDIA, "C", "DirectMedia")), "Pic");
    assert_eq!(text(al_testkit::run_in(MEDIA, "C", "DirectMediaSet")), "Pics");
    assert_eq!(text(al_testkit::run_in(MEDIA, "C", "ThroughCopy")), "Id");
    assert_eq!(text(al_testkit::run_in(MEDIA, "C", "ResetClears")), "Id3");
    for p in ["FindsInMediaKey", "ViewOfMediaKey", "FindsOnCopy"] {
        let r = al_testkit::run_in(MEDIA, "C", p);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{p}: {r:?}");
    }
}
