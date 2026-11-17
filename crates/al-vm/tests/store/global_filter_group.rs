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
