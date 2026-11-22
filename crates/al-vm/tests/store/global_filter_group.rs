//! A filter set in filter group 1 (the global group) stays on every record of the table for the rest of the
//! session: a later record of the table counts and deletes with it, also after Reset and Clear
//! (types/text-code-char/P145PageRunView.al:232-235, SfSeed, and the SF_ tests from :271 on).
//! A record that holds no group 1 filter reads the session's for the calls and the filter form
//! record/filters/FilterGroupOtherVariable.al measured (al-vm `group1_sessions.rs`); every other shape keeps main's
//! refusal (hard rule 4).

use al_builtins::native::Signal;
use al_testkit::{Built, int};

const SRC: &str = r#"
table 70950 "LT Glob Row"
{
    fields { field(1; No; Integer) { } field(2; Qty; Integer) { } }
    keys { key(PK; No) { } }
}
codeunit 70951 C
{
    procedure Seed()
    var R: Record "LT Glob Row"; I: Integer;
    begin
        for I := 1 to 6 do begin
            R.No := I; R.Qty := I; R.Insert();
        end;
    end;

    procedure SetGlobal(): Integer
    var R: Record "LT Glob Row";
    begin
        R.FilterGroup(1);
        R.SetFilter(Qty, '>=3');
        R.FilterGroup(0);
        exit(R.Count());
    end;

    procedure SetAndClearGlobal(): Integer
    var R: Record "LT Glob Row";
    begin
        R.FilterGroup(1);
        R.SetFilter(Qty, '>=3');
        R.FilterGroup(0);
        R.FilterGroup(1);
        R.SetRange(Qty);
        R.FilterGroup(0);
        exit(R.Count());
    end;

    procedure CountFresh(): Integer
    var R: Record "LT Glob Row";
    begin
        R.Reset();
        exit(R.Count());
    end;

    procedure SetOne(): Integer
    var R: Record "LT Glob Row";
    begin
        R.FilterGroup(1);
        R.SetRange(Qty, 3);
        R.FilterGroup(0);
        exit(R.Count());
    end;

    procedure ClearByFresh()
    var R: Record "LT Glob Row";
    begin
        R.FilterGroup(1);
        R.SetRange(Qty);
        R.FilterGroup(0);
    end;

    procedure SetOtherByFresh()
    var R: Record "LT Glob Row";
    begin
        R.FilterGroup(1);
        R.SetRange(Qty, 4);
        R.FilterGroup(0);
    end;

    procedure FindFirstFresh(): Integer
    var R: Record "LT Glob Row";
    begin
        R.FindFirst();
        exit(R.No);
    end;

    procedure SetterAfterFreshTookItBack(): Integer
    var R: Record "LT Glob Row";
    begin
        R.FilterGroup(1);
        R.SetRange(Qty, 3);
        R.FilterGroup(0);
        ClearByFresh();
        exit(R.Count());
    end;

    procedure SetFilterOnTemporary()
    var R: Record "LT Glob Row" temporary;
    begin
        R.FilterGroup(1);
        R.SetFilter(Qty, '3');
        R.FilterGroup(0);
    end;

    procedure RefNumberAndName(): Text
    var RR: RecordRef;
    begin
        RR.Open(70950);
        exit(Format(RR.Number()) + ' ' + RR.Name());
    end;

    procedure CountTemporary(): Integer
    var R: Record "LT Glob Row" temporary;
    begin
        exit(R.Count());
    end;
}
"#;

fn unsupported(r: Result<al_builtins::value::Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(why)) => why.into_reason(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn the_record_that_set_the_global_filter_counts_with_it() {
    let mut b = Built::from_source(SRC);
    b.run_in("C", "Seed").unwrap();
    assert_eq!(int(b.run_in("C", "SetGlobal")), 4);
}

#[test]
fn a_later_record_of_the_table_is_refused_while_the_global_filter_stands() {
    // BC counts 4 here (P145PageRunView.al:246, seed_before = 4); the runtime would count 6.
    let mut b = Built::from_source(SRC);
    b.run_in("C", "Seed").unwrap();
    b.run_in("C", "SetGlobal").unwrap();
    let why = unsupported(b.run_in("C", "CountFresh"));
    assert!(why.contains("filter group 1"), "{why}");
    let why = unsupported(b.run_in("C", "CountTemporary"));
    assert!(why.contains("filter group 1"), "{why}");
}

#[test]
fn a_global_filter_cleared_again_leaves_later_records_answered() {
    // record/crud/RcAFilterInfo.al:186-208 clears its group 1 filter before the test ends.
    let mut b = Built::from_source(SRC);
    b.run_in("C", "Seed").unwrap();
    assert_eq!(int(b.run_in("C", "SetAndClearGlobal")), 6);
    assert_eq!(int(b.run_in("C", "CountFresh")), 6);
}

#[test]
fn a_later_record_reads_a_single_value_filter_only_for_the_measured_calls() {
    let mut b = Built::from_source(SRC);
    b.run_in("C", "Seed").unwrap();
    assert_eq!(int(b.run_in("C", "SetOne")), 1);
    assert_eq!(int(b.run_in("C", "CountFresh")), 1);
    // FindFirst is not among the measured calls: main's refusal.
    let why = unsupported(b.run_in("C", "FindFirstFresh"));
    assert!(why.contains("filter group 1"), "{why}");
    // A later record that sets another value in group 1: main's refusal.
    let why = unsupported(b.run_in("C", "SetOtherByFresh"));
    assert!(why.contains("filter group 1"), "{why}");
    // Taken back by a record that held none: a later record reads every row.
    b.run_in("C", "ClearByFresh").unwrap();
    assert_eq!(int(b.run_in("C", "CountFresh")), 6);
}

#[test]
fn a_record_still_holding_the_filter_after_another_took_it_back_is_refused() {
    let mut b = Built::from_source(SRC);
    b.run_in("C", "Seed").unwrap();
    let why = unsupported(b.run_in("C", "SetterAfterFreshTookItBack"));
    assert!(why.contains("took the global group's filter back"), "{why}");
}

#[test]
fn setfilter_in_group_1_on_a_temporary_record_is_refused() {
    let mut b = Built::from_source(SRC);
    let why = unsupported(b.run_in("C", "SetFilterOnTemporary"));
    assert!(why.contains("on a temporary record"), "{why}");
}

/// The other-variable shapes of al-runner issue 7586 item 3 that FilterGroupOtherVariable.al did not measure: each flips one
/// fact of a measured shape and keeps main's refusal.
const OTHER: &str = r#"
table 70960 "LT G1 Plain"
{
    fields { field(1; Id; Integer) { } field(2; Tag; Integer) { } }
    keys { key(PK; Id) { } }
}
table 70961 "LT G1 Subscribed"
{
    fields { field(1; Id; Integer) { } field(2; Tag; Integer) { } }
    keys { key(PK; Id) { } }
}
codeunit 70962 "LT G1 Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"LT G1 Subscribed", 'OnBeforeDeleteEvent', '', false, false)]
    local procedure OnBeforeDelete(var Rec: Record "LT G1 Subscribed"; RunTrigger: Boolean)
    begin
    end;
}
codeunit 70963 G
{
    local procedure Seed()
    var P: Record "LT G1 Plain"; S: Record "LT G1 Subscribed"; I: Integer;
    begin
        for I := 1 to 4 do begin
            P.Id := I; P.Tag := I mod 2; P.Insert();
            S.Id := I; S.Tag := I mod 2; S.Insert();
        end;
    end;

    local procedure SetTag()
    var A: Record "LT G1 Plain";
    begin
        A.FilterGroup(1); A.SetRange(Tag, 1); A.FilterGroup(0);
    end;

    procedure Control(): Integer
    var B: Record "LT G1 Plain"; N: Integer;
    begin
        Seed(); SetTag();
        if B.FindSet() then
            repeat N += 1; until B.Next() = 0;
        exit(B.Count() * 10 + N);
    end;

    procedure GroupZeroFilter(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); SetTag(); B.SetRange(Id, 1, 4); exit(B.Count());
    end;

    procedure FilterBeforeTheSet(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); B.SetRange(Id, 1, 4); SetTag(); exit(B.Count());
    end;

    procedure GroupTwoFilter(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); SetTag(); B.FilterGroup(2); B.SetRange(Id, 1, 4); B.FilterGroup(0); exit(B.Count());
    end;

    procedure GroupZeroFilterDeleteAll(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); SetTag(); B.SetRange(Id, 1, 2); B.DeleteAll(); exit(0);
    end;

    procedure MarkedOnly(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); SetTag(); B.MarkedOnly(true); exit(B.Count());
    end;

    procedure Descending(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); SetTag(); B.Ascending(false); B.FindSet(); exit(B.Id);
    end;

    procedure OtherKey(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); SetTag(); B.SetCurrentKey(Tag); exit(B.Count());
    end;

    procedure ReadInGroupTwo(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); SetTag(); B.FilterGroup(2); exit(B.Count());
    end;

    procedure DeleteAllTrue(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); SetTag(); B.DeleteAll(true); exit(0);
    end;

    procedure InsertTrue(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); SetTag(); B.Id := 9; B.Tag := 2; B.Insert(true); exit(0);
    end;

    procedure ModifyAllTrue(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); SetTag(); B.ModifyAll(Tag, 7, true); exit(0);
    end;

    procedure DeleteAllSubscribed(): Integer
    var A: Record "LT G1 Subscribed"; B: Record "LT G1 Subscribed";
    begin
        Seed(); A.FilterGroup(1); A.SetRange(Tag, 1); A.FilterGroup(0); B.DeleteAll(); exit(0);
    end;

    procedure NextTwo(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); SetTag(); B.FindSet(); exit(B.Next(2));
    end;

    procedure FindSetForUpdate(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); SetTag(); B.FindSet(true); exit(B.Id);
    end;

    procedure KeyFieldFilter(): Integer
    var A: Record "LT G1 Plain"; B: Record "LT G1 Plain";
    begin
        Seed(); A.FilterGroup(1); A.SetRange(Id, 1); A.FilterGroup(0); exit(B.Count());
    end;

    procedure TemporarySetterDeleteAll(): Integer
    var T: Record "LT G1 Plain" temporary; B: Record "LT G1 Plain";
    begin
        Seed(); T.FilterGroup(1); T.SetRange(Tag, 1); T.FilterGroup(0); B.DeleteAll(); exit(0);
    end;

    procedure TemporarySetterRecordRefCount(): Integer
    var T: Record "LT G1 Plain" temporary; Other: RecordRef;
    begin
        Seed(); T.FilterGroup(1); T.SetRange(Tag, 1); T.FilterGroup(0);
        Other.Open(Database::"LT G1 Plain"); exit(Other.Count());
    end;

    procedure RecordRefSetterTemporaryCount(): Integer
    var Setter: RecordRef; TagRef: FieldRef; T: Record "LT G1 Plain" temporary;
    begin
        Seed(); Setter.Open(Database::"LT G1 Plain"); Setter.FilterGroup(1); TagRef := Setter.Field(2); TagRef.SetRange(1);
        Setter.FilterGroup(0); T.Id := 1; T.Tag := 1; T.Insert(); exit(T.Count());
    end;

    procedure RecordRefSetterDeleteAll(): Integer
    var Setter: RecordRef; TagRef: FieldRef; B: Record "LT G1 Plain";
    begin
        Seed(); Setter.Open(Database::"LT G1 Plain"); Setter.FilterGroup(1); TagRef := Setter.Field(2); TagRef.SetRange(1);
        Setter.FilterGroup(0); B.DeleteAll(); exit(0);
    end;

    procedure SetInsideFindSetLoop(): Integer
    var B: Record "LT G1 Plain";
    begin
        Seed(); B.FindSet(); SetTag(); B.Next(); exit(B.Id);
    end;
}
"#;

fn refused_in_other(proc_name: &str) {
    let mut b = Built::from_source(OTHER);
    let why = unsupported(b.run_in("G", proc_name));
    assert!(why.contains("filter group 1"), "{proc_name}: {why}");
}

#[test]
fn a_reader_with_no_filter_of_its_own_reads_the_shared_filter_for_the_measured_calls() {
    // FilterGroupOtherVariable.al:163-174, :605-611: two rows of four, by Count and by a FindSet loop.
    let mut b = Built::from_source(OTHER);
    assert_eq!(int(b.run_in("G", "Control")), 22);
}

#[test]
fn a_reader_with_a_filter_or_view_of_its_own_keeps_main_s_refusal() {
    for p in [
        "GroupZeroFilter",
        "FilterBeforeTheSet",
        "GroupTwoFilter",
        "GroupZeroFilterDeleteAll",
        "MarkedOnly",
        "Descending",
        "OtherKey",
    ] {
        refused_in_other(p);
    }
}

#[test]
fn a_reader_in_another_current_filter_group_keeps_main_s_refusal() {
    refused_in_other("ReadInGroupTwo");
}

#[test]
fn a_shared_write_with_run_trigger_keeps_main_s_refusal() {
    for p in ["DeleteAllTrue", "InsertTrue", "ModifyAllTrue"] {
        refused_in_other(p);
    }
}

#[test]
fn a_shared_write_on_a_table_with_an_event_subscriber_keeps_main_s_refusal() {
    refused_in_other("DeleteAllSubscribed");
}

#[test]
fn a_shared_read_with_other_arguments_keeps_main_s_refusal() {
    for p in ["NextTwo", "FindSetForUpdate"] {
        refused_in_other(p);
    }
}

#[test]
fn a_shared_filter_on_a_primary_key_field_keeps_main_s_refusal() {
    refused_in_other("KeyFieldFilter");
}

#[test]
fn a_setter_and_reader_kind_pair_the_probe_did_not_measure_keeps_main_s_refusal() {
    for p in [
        "TemporarySetterDeleteAll",
        "TemporarySetterRecordRefCount",
        "RecordRefSetterTemporaryCount",
        "RecordRefSetterDeleteAll",
    ] {
        refused_in_other(p);
    }
}

#[test]
fn next_on_a_reader_whose_find_set_ran_before_the_filter_keeps_main_s_refusal() {
    refused_in_other("SetInsideFindSetLoop");
}

// aldevtools pull request 7874 let a RecordRef answer Number, Name and other metadata while another record's group 1 filter
// stands. No corpus case measures a RecordRef call on such a table other than Count and GetFilters
// (record/filters/FilterGroupOtherVariable.al:859, :896-898), so every other call keeps the refusal until a probe does.
#[test]
fn a_record_ref_metadata_call_keeps_main_s_refusal_while_the_global_filter_stands() {
    let mut b = Built::from_source(SRC);
    b.run_in("C", "Seed").unwrap();
    b.run_in("C", "SetGlobal").unwrap();
    let why = unsupported(b.run_in("C", "RefNumberAndName"));
    assert!(
        why.contains("RecordRef.Number") && why.contains("filter group 1"),
        "{why}"
    );
}
