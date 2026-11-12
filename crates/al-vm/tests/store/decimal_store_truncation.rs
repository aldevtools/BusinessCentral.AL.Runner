//! The SQL `decimal(38,20)` column behind a Decimal field of a database table holds 20 decimals: a value written cuts the
//! rest off toward zero, and Get reads it back so, while the record that was inserted keeps its own value. Measured on real
//! BC 28.5 (docs/decimal-store-probe.md).

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
    keys { key(PK; Id) { } }
}
table 69961 "DS Decimal Key"
{
    fields
    {
        field(1; Amount; Decimal) { }
    }
    keys { key(PK; Amount) { } }
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

    procedure DecimalKeyTruncated()
    var R: Record "DS Decimal Key";
    begin
        R.Amount := 1000 / 3600000; R.Insert();
    end;

    procedure DecimalKeyExact(): Text
    var R: Record "DS Decimal Key";
    begin
        R.Amount := 0.5; R.Insert();
        exit(Format(R.Get(0.5)));
    end;
}
"#;

fn run(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

// docs/decimal-store-probe.md: 1000/3600000 reads back as 0.00027777777777777777, -1000/3600000 as -0.00027777777777777777,
// 0.000000000000000000019 as 0.00000000000000000001, 0.00012345678901234567891 as 0.00012345678901234567; the record that
// was inserted keeps 0.000277777777777777778.
#[test]
fn insert_cuts_a_decimal_to_twenty_places() {
    assert_eq!(text(run("InsertThenGet")), "Yes,Yes,No,Yes,Yes,Yes");
}

#[test]
fn modify_cuts_a_decimal_to_twenty_places() {
    assert_eq!(text(run("ModifyThenGet")), "Yes");
}

// Not pinned: a Decimal primary key value the column cuts.
#[test]
fn decimal_key_the_column_cuts_is_refused() {
    assert!(
        matches!(run("DecimalKeyTruncated"), Err(Signal::Unsupported(_))),
        "{:?}",
        run("DecimalKeyTruncated")
    );
}

#[test]
fn decimal_key_the_column_holds_is_stored() {
    assert_eq!(text(run("DecimalKeyExact")), "Yes");
}
