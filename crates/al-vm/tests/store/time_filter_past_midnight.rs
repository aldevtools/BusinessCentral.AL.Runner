//! A Time filter value on a database table that the SQL `datetime` column rounds past the day (23:59:59.999 goes on to the
//! next day) stays after every time of the day: `< 23:59:59.999` finds a row at 00:00:00. Measured on real BC 28.5
//! (docs/time-filter-probe.md).

use al_testkit::text;

const SRC: &str = r#"
table 69970 "TF Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Tm; Time) { }
    }
    keys { key(PK; Id) { } }
}
codeunit 69971 C
{
    procedure Filters(): Text
    var R: Record "TF Row"; s: Text;
    begin
        Ins(1, 000000T); Ins(2, 120000T); Ins(3, 235959.997T); Ins(5, 000000.001T);
        R.SetFilter(Tm, '<%1', 235959.999T); s += 'lt999=' + Ids(R) + ';';
        R.SetFilter(Tm, '<=%1', 235959.999T); s += 'le999=' + Ids(R) + ';';
        R.SetFilter(Tm, '>%1', 235959.999T); s += 'gt999=' + Ids(R) + ';';
        R.SetFilter(Tm, '>=%1', 235959.999T); s += 'ge999=' + Ids(R) + ';';
        R.SetRange(Tm, 235959.999T); s += 'eq999=' + Ids(R) + ';';
        R.SetRange(Tm, 000000T, 235959.999T); s += 'rng0to999=' + Ids(R) + ';';
        R.SetRange(Tm, 120000T, 235959.999T); s += 'rng12to999=' + Ids(R) + ';';
        R.SetFilter(Tm, '<>%1', 235959.999T); s += 'ne999=' + Ids(R) + ';';
        R.SetFilter(Tm, '<%1', 235959.998T); s += 'lt998=' + Ids(R) + ';';
        R.SetFilter(Tm, '>%1', 000000T); s += 'gt0=' + Ids(R) + ';';
        R.SetFilter(Tm, '<%1', 235959.999T); s += 'filters=' + R.GetFilters();
        exit(s);
    end;

    local procedure Ins(K: Integer; T: Time)
    var R: Record "TF Row";
    begin
        R.Id := K; R.Tm := T; R.Insert();
    end;

    local procedure Ids(var R: Record "TF Row") s: Text
    begin
        if R.FindSet() then
            repeat
                s += Format(R.Id);
            until R.Next() = 0;
    end;
}
"#;

// docs/time-filter-probe.md: with rows 1 (00:00:00), 2 (12:00:00), 3 (23:59:59.997) and 5 (00:00:00.001, stored as
// 00:00:00) real BC gives lt999=1235, le999=12345, gt999=, ge999=4, eq999=4, rng0to999=12345, rng12to999=234, ne999=1235,
// lt998=125, gt0=234 (row 4 holds 23:59:59.999 and is left out here).
#[test]
fn time_filter_rounded_past_midnight_stays_after_the_day() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Filters")),
        "lt999=1235;le999=1235;gt999=;ge999=;eq999=;rng0to999=1235;rng12to999=23;ne999=1235;lt998=125;gt0=23;\
         filters=Tm: <11:59:59.999 PM"
    );
}
