//! `DMY2Date(Day)` and `DMY2Date(Day, Month)` take the missing month and year from Today, not from the work date, and the
//! day's range is the length of that month (errors/lt-ms-refusals-133/P133DateEvaluateFormat.al:104-218).

use al_testkit::{run, text};

const SRC: &str = r#"
enum 50131 "DM Month" { value(0; None) { } value(1; January) { } value(2; February) { } value(3; March) { } }
table 50132 "DM Row"
{
    fields { field(1; Id; Integer) { } field(2; Day; Integer) { } field(3; Month; Enum "DM Month") { } field(4; Start; Date) { } }
    keys { key(PK; Id) { } }
}
codeunit 50130 C
{
    // System Application, Recurrence Schedule Impl.: DMY2Date(Rec.Day, Rec.Month.AsInteger(), Date2DMY(Rec.Start, 3)).
    procedure FieldArgs(): Text
    var
        R: Record "DM Row";
    begin
        R.Day := 5;
        R.Month := R.Month::March;
        R.Start := DMY2Date(1, 1, 2024);
        exit(Format(DMY2Date(R.Day, R.Month.AsInteger(), Date2DMY(R.Start, 3)), 0, 9));
    end;

    [TryFunction]
    local procedure TryDay(Day: Integer; var Result: Date)
    begin
        Result := DMY2Date(Day);
    end;

    [TryFunction]
    local procedure TryDayMonth(Day: Integer; Month: Integer; var Result: Date)
    begin
        Result := DMY2Date(Day, Month);
    end;

    procedure Forms(): Text
    begin
        WorkDate(DMY2Date(10, 2, 2000));
        exit(Format(DMY2Date(15) = DMY2Date(15, Date2DMY(Today(), 2), Date2DMY(Today(), 3))) + '|' +
             Format(DMY2Date(15, 3) = DMY2Date(15, 3, Date2DMY(Today(), 3))));
    end;

    procedure Errors(): Text
    var
        D: Date;
        R: Text;
    begin
        if not TryDay(32, D) then
            R := GetLastErrorText().Replace(Format(Date2DMY(CalcDate('<CM>', Today()), 1)), '<last>');
        if not TryDayMonth(15, 13, D) then
            R += '|' + GetLastErrorText();
        if not TryDayMonth(31, 4, D) then
            R += '|' + GetLastErrorText();
        exit(R);
    end;
}
"#;

#[test]
fn the_missing_month_and_year_come_from_today() {
    assert_eq!(text(run(SRC, "Forms")), "Yes|Yes");
}

#[test]
fn the_short_forms_name_the_part_out_of_range() {
    assert_eq!(
        text(run(SRC, "Errors")),
        "The value of DMY2Date parameter 1 is outside of the permitted range. The current value is: 32. The permitted range is: from 1 to <last>.\
         |The value of DMY2Date parameter 2 is outside of the permitted range. The current value is: 13. The permitted range is: from 1 to 12.\
         |The value of DMY2Date parameter 1 is outside of the permitted range. The current value is: 31. The permitted range is: from 1 to 30."
    );
}

#[test]
fn an_enum_ordinal_and_an_integer_field_are_integers() {
    assert_eq!(text(run(SRC, "FieldArgs")), "2024-03-05");
}
