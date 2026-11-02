//! `Record.CalcSums` on a Duration field (record/lt-calcsums-duration/CalcSumsDuration.al CalcSums_Duration_All, _Filtered,
//! _NotInSumIndexFields): the sum in milliseconds, also for a field that is in no SumIndexFields, an empty set gives 0, and
//! the other fields of the record stay as they were.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50980 "Dur Row"
{
    fields { field(1; Id; Integer) { } field(2; Dur; Duration) { } field(3; Dec; Decimal) { } field(4; Plain; Duration) { } }
    keys { key(PK; Id) { Clustered = true; SumIndexFields = Dur, Dec; } }
}
codeunit 50980 C
{
    local procedure Seed()
    var Row: Record "Dur Row"; i: Integer;
    begin
        Row.DeleteAll();
        for i := 1 to 3 do begin
            Row.Init(); Row.Id := i; Row.Dur := i * 1000; Row.Dec := i; Row.Plain := i * 60000; Row.Insert();
        end;
    end;

    procedure All(): Text
    var Row: Record "Dur Row";
    begin
        Seed();
        Row.Id := 99; Row.Dec := 5; Row.Plain := 888;
        Row.CalcSums(Dur);
        exit(Format(Row.Dur, 0, 9) + '|' + Format(Row.Id) + '|' + Format(Row.Dec) + '|' + Format(Row.Plain, 0, 9));
    end;

    procedure Filtered(): Text
    var Row: Record "Dur Row";
    begin
        Seed();
        Row.SetRange(Id, 2, 3);
        Row.CalcSums(Dur, Dec);
        exit(Format(Row.Dur, 0, 9) + '|' + Format(Row.Dec));
    end;

    procedure PlainField(): Text
    var Row: Record "Dur Row";
    begin
        Seed();
        Row.CalcSums(Plain);
        exit(Format(Row.Plain, 0, 9));
    end;
}
"#;

#[test]
fn calc_sums_adds_a_duration_field() {
    assert_eq!(text(run_in(SRC, "C", "All")), "P0DT0H0M6.0S|99|5|P0DT0H0M0.888S");
    assert_eq!(text(run_in(SRC, "C", "Filtered")), "P0DT0H0M5.0S|5");
    assert_eq!(text(run_in(SRC, "C", "PlainField")), "P0DT0H6M0.0S");
}
