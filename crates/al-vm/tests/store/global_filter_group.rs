//! A filter set in filter group 1 (the global group) stays on every record of the table for the rest of the
//! session: a later record of the table counts and deletes with it, also after Reset and Clear
//! (types/text-code-char/P145PageRunView.al:232-235, SfSeed, and the SF_ tests from :271 on).
//! The runtime keeps filters on the variable that set them, so a record of the table whose group 1 differs
//! from the session's is refused (hard rule 4) until the global group is modelled.

use al_builtins::native::Signal;
use al_testkit::{Built, int, text};

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

    procedure CountTemporary(): Integer
    var R: Record "LT Glob Row" temporary;
    begin
        exit(R.Count());
    end;
    procedure RefNumberAndName(): Text
    var RR: RecordRef;
    begin
        RR.Open(70950);
        exit(Format(RR.Number()) + ' ' + RR.Name());
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
fn a_record_ref_answers_its_metadata_while_the_global_filter_stands() {
    let mut b = Built::from_source(SRC);
    b.run_in("C", "Seed").unwrap();
    b.run_in("C", "SetGlobal").unwrap();
    assert_eq!(text(b.run_in("C", "RefNumberAndName")), "70950 LT Glob Row");
}
