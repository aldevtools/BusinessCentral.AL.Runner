//! The SQL `decimal(38,20)` column behind a Decimal field of a database table cuts a value toward zero after 20 decimals,
//! while the record that was written keeps its own value. Assumption `decimal-column-cuts-20-places` (open corpus pull request
//! 1427, record/crud/RcAGetFind.al:749-842): only the measured field shape (a field with one secondary key of its own), the
//! measured writers (Record.Insert(), Record.Modify()) and the measured filter reads (Record.Count() after Record.SetRange or
//! Record.SetFilter('<%1' or '>%1')). Other writers and readers of such a field refuse; other field shapes keep the values.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69960 "DS Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { }
    }
    keys { key(PK; Id) { } key(ByAmount; Amount) { } }
}
table 69961 "DS Decimal Key"
{
    fields
    {
        field(1; Amount; Decimal) { }
    }
    keys { key(PK; Amount) { } }
}
table 69963 "DS Plain"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { }
    }
    keys { key(PK; Id) { } }
}
codeunit 69962 C
{
    procedure InsertThenGet(): Text
    var R: Record "DS Row"; G: Record "DS Row"; s: Text;
    begin
        R.Id := 1; R.Amount := 1000 / 3600000; R.Insert();
        G.Get(1);
        s := Format(G.Amount = 0.00027777777777777777);
        s += ',' + Format(R.Amount = 1000 / 3600000);
        s += ',' + Format(R.Amount = G.Amount);
        R.Id := 2; R.Amount := -1000 / 3600000; R.Insert();
        G.Get(2);
        s += ',' + Format(G.Amount = -0.00027777777777777777);
        R.Id := 3; R.Amount := 0.000000000000000000019; R.Insert();
        G.Get(3);
        s += ',' + Format(G.Amount = 0.00000000000000000001);
        R.Id := 4; R.Amount := 0.00012345678901234567891; R.Insert();
        G.Get(4);
        s += ',' + Format(G.Amount = 0.00012345678901234567);
        exit(s);
    end;

    procedure ModifyThenGet(): Text
    var R: Record "DS Row"; G: Record "DS Row";
    begin
        R.Id := 1; R.Insert();
        R.Amount := 1000 / 3600000; R.Modify();
        G.Get(1);
        exit(Format(G.Amount = 0.00027777777777777777));
    end;

    procedure FilterCounts(): Text
    var R: Record "DS Row"; Full: Decimal; Cut: Decimal; s: Text;
    begin
        Full := 1000 / 3600000; Cut := 0.00027777777777777777;
        R.Id := 1; R.Amount := Full; R.Insert();
        R.SetRange(Amount, Full); s := Format(R.Count());
        R.SetRange(Amount, Cut); s += Format(R.Count());
        R.SetFilter(Amount, '<%1', Full); s += Format(R.Count());
        R.SetFilter(Amount, '>%1', Cut); s += Format(R.Count());
        R.SetRange(Amount, Cut, Full); s += Format(R.Count());
        exit(s);
    end;

    procedure FindThroughCutFilter()
    var R: Record "DS Row";
    begin
        R.Id := 1; R.Amount := 1000 / 3600000; R.Insert();
        R.SetRange(Amount, 1000 / 3600000);
        R.FindFirst();
    end;

    procedure FieldRefSetRangeCount()
    var R: Record "DS Row"; RR: RecordRef; FR: FieldRef; n: Integer;
    begin
        RR.Open(69960);
        FR := RR.Field(2);
        FR.SetRange(1000 / 3600000);
        RR.SetTable(R);
        n := R.Count();
    end;

    procedure InsertWithTrigger()
    var R: Record "DS Row";
    begin
        R.Id := 1; R.Amount := 1000 / 3600000; R.Insert(true);
    end;

    procedure ModifyAllCut()
    var R: Record "DS Row";
    begin
        R.Id := 1; R.Insert();
        R.ModifyAll(Amount, 1000 / 3600000);
    end;

    procedure PlainFieldKeepsValue(): Text
    var R: Record "DS Plain"; G: Record "DS Plain";
    begin
        R.Id := 1; R.Amount := 1000 / 3600000; R.Insert();
        G.Get(1);
        exit(Format(G.Amount = 1000 / 3600000));
    end;

    procedure DecimalKeyKeepsValue(): Text
    var R: Record "DS Decimal Key";
    begin
        R.Amount := 1000 / 3600000; R.Insert();
        exit(Format(R.Get(1000 / 3600000)));
    end;
}
"#;

fn run(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

fn refused(routine: &str) {
    let got = run(routine);
    assert!(matches!(got, Err(Signal::Unsupported(_))), "{routine}: {got:?}");
}

// RcAGetFind.al:749-778: 1000/3600000 reads back as 0.00027777777777777777, -1000/3600000 as -0.00027777777777777777,
// 0.000000000000000000019 as 0.00000000000000000001, 0.00012345678901234567891 as 0.00012345678901234567; the record that
// was inserted keeps its own value.
#[test]
fn insert_cuts_a_decimal_to_twenty_places() {
    clear_hits();
    assert_eq!(text(run("InsertThenGet")), "Yes,Yes,No,Yes,Yes,Yes");
    assert!(take_hits().contains(&Assumption::DecimalColumnCuts20Places));
}

// RcAGetFind.al:780-799.
#[test]
fn modify_cuts_a_decimal_to_twenty_places() {
    clear_hits();
    assert_eq!(text(run("ModifyThenGet")), "Yes");
    assert!(take_hits().contains(&Assumption::DecimalColumnCuts20Places));
}

// RcAGetFind.al:801-842: equal_full 1, equal_cut 1, less_full 0, greater_cut 0, range_cut_full 1.
#[test]
fn count_compares_filter_values_cut() {
    assert_eq!(text(run("FilterCounts")), "11001");
}

#[test]
fn unmeasured_readers_writers_and_setters_of_the_field_are_refused() {
    refused("FindThroughCutFilter");
    refused("FieldRefSetRangeCount");
    refused("InsertWithTrigger");
    refused("ModifyAllCut");
}

// Other field shapes keep the value as main did: a field without a key, a Decimal primary key.
#[test]
fn other_field_shapes_keep_the_value() {
    clear_hits();
    assert_eq!(text(run("PlainFieldKeepsValue")), "Yes");
    assert_eq!(text(run("DecimalKeyKeepsValue")), "Yes");
    assert!(!take_hits().contains(&Assumption::DecimalColumnCuts20Places));
}
