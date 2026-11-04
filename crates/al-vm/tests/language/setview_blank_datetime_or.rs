//! A blank DateTime next to another alternative in a view reads back as `''..''`, a blank Date as `''`
//! (record/views/Y71SetViewDateTime.al:125, :289).

use al_testkit::text;

const SRC: &str = r#"
table 69730 "SVB Row" { fields { field(1; Id; Integer) { } field(2; D; Date) { } field(3; Dt; DateTime) { } } keys { key(PK; Id) { } } }
codeunit 69731 C
{
    procedure F(): Text
    var R: Record "SVB Row";
    begin
        R.SetView('WHERE(Field3=1(''''|2025-11-01))');
        exit(R.GetView(false));
    end;
    procedure G(): Text
    var R: Record "SVB Row";
    begin
        R.SetView('WHERE(Field2=1(''''|2025-11-01))');
        exit(R.GetView(false));
    end;
}
"#;

#[test]
fn blank_datetime_alternative_is_a_range() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "F")),
        "VERSION(1) SORTING(Field1) WHERE(Field3=1(''..''|2025-11-01T00:00:00Z..2025-11-01T23:59:59Z))"
    );
}

#[test]
fn blank_date_alternative_stays_quotes() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "G")),
        "VERSION(1) SORTING(Field1) WHERE(Field2=1(''|2025-11-01))"
    );
}
