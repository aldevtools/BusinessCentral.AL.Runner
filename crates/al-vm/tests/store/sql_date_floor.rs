//! A Date or DateTime in the year 1753 written to a database table (assumption `sql-date-first-year`, al-runner issue 8006:
//! the reporter's measurement on BC 28.4). The pinned Insert cells stay clean (record/filters/P130DateFilters.al:255-293).

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const SRC: &str = r#"
table 69990 "SDF Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Dt; Date) { }
        field(3; Tm; Time) { }
        field(4; Dtm; DateTime) { }
    }
    keys { key(PK; Id) { } }
}
table 69991 "SDF Trig Row"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } }
    keys { key(PK; Id) { } }
    trigger OnModify() begin end;
}
codeunit 69992 C
{
    // The reporter's Insert shape, as record/filters/P130DateFilters.al:234-251 writes it.
    procedure Insert(Dt: Date; Dtm: DateTime): Text
    var R: Record "SDF Row"; Back: Record "SDF Row"; Stage: Text;
    begin
        ClearLastError();
        asserterror begin
            Stage := 'insert';
            R.Init(); R.Id := 900; R.Dt := Dt; R.Dtm := Dtm; R.Insert();
            Stage := 'get';
            Back.Get(900);
            Stage := 'stored';
            Error('stored:' + Format(Back.Dt, 0, 9) + '|' + Format(Back.Dtm, 0, 9));
        end;
        exit(Stage + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    // The reporter's Modify shape: Modify of an inserted row.
    procedure Modify(Dt: Date; Dtm: DateTime): Text
    var R: Record "SDF Row"; Back: Record "SDF Row"; Stage: Text;
    begin
        R.Init(); R.Id := 900; R.Insert();
        ClearLastError();
        asserterror begin
            Stage := 'modify';
            R.Dt := Dt; R.Dtm := Dtm; R.Modify();
            Stage := 'get';
            Back.Get(900);
            Stage := 'stored';
            Error('stored:' + Format(Back.Dt, 0, 9) + '|' + Format(Back.Dtm, 0, 9));
        end;
        exit(Stage + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure TempInsertModify(Dt: Date): Text
    var R: Record "SDF Row" temporary;
    begin
        R.Id := 1; R.Dt := Dt; R.Insert();
        R.Id := 2; R.Insert(); R.Dt := Dt; R.Modify();
        R.Get(1);
        exit(Format(R.Dt, 0, 9));
    end;

    procedure ModifyUsed(Dt: Date): Boolean
    var R: Record "SDF Row";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt;
        exit(R.Modify());
    end;

    procedure ModifyWithTrigger(Dt: Date)
    var R: Record "SDF Trig Row";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Modify(true);
    end;

    procedure RenameTo(Dt: Date)
    var R: Record "SDF Row";
    begin
        R.Id := 1; R.Insert();
        R.Dt := Dt; R.Rename(2);
    end;
}
"#;

fn date(y: i32, m: u32, d: u32, closing: bool) -> Value {
    Value::Date(al_builtins::format::day_from_civil(y, m, d), closing)
}

fn datetime(y: i32, m: u32, d: u32, ms: i64) -> Value {
    Value::DateTime((i64::from(al_builtins::format::day_from_civil(y, m, d)) - 1) * 86_400_000 + ms + 1)
}

const BLANK_DT: Value = Value::Date(0, false);
const BLANK_DTM: Value = Value::DateTime(0);

/// The routine's result and whether the run passed through the assumption.
fn run(routine: &str, args: Vec<Value>) -> (Result<Value, Signal>, bool) {
    clear_hits();
    let r = al_testkit::run_with(SRC, "C", routine, args).map(|r| r.0);
    (r, take_hits().contains(&Assumption::SqlDateFirstYear))
}

fn flagged(routine: &str, args: Vec<Value>) -> String {
    let (r, hit) = run(routine, args);
    assert!(hit, "{routine} did not pass through the assumption: {r:?}");
    text(r)
}

fn unflagged(routine: &str, args: Vec<Value>) -> String {
    let (r, hit) = run(routine, args);
    assert!(!hit, "{routine} passed through the assumption: {r:?}");
    text(r)
}

fn refused(routine: &str, args: Vec<Value>) {
    let (r, _) = run(routine, args);
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
}

fn bad(stage: &str, value: &str, field: &str) -> String {
    format!(
        "{stage}|Cannot write the value {value} to the field {field} in the table SDF Row because the value is either too long or the content is invalid.|NDBCS:BadCSideData"
    )
}

// al-runner issue 8006: the Insert fails as the pinned values do, at the next access (P130DateFilters.al:255-275).
#[test]
fn an_insert_of_a_1753_date_fails_at_the_next_access() {
    assert_eq!(
        flagged("Insert", vec![date(1753, 1, 2, false), BLANK_DTM]),
        bad("get", "01/02/1753", "Dt")
    );
    assert_eq!(
        flagged("Insert", vec![date(1753, 12, 31, false), BLANK_DTM]),
        bad("get", "12/31/1753", "Dt")
    );
    assert_eq!(
        flagged("Insert", vec![BLANK_DT, datetime(1753, 12, 31, 86_399_000)]),
        bad("get", "12/31/1753 11:59 PM", "Dtm")
    );
    assert_eq!(
        flagged("Insert", vec![BLANK_DT, datetime(1753, 1, 1, 1_000)]),
        bad("get", "01/01/1753 12:00 AM", "Dtm")
    );
    assert_eq!(
        flagged("Insert", vec![BLANK_DT, datetime(1753, 1, 4, 12 * 3_600_000)]),
        bad("get", "01/04/1753 12:00 PM", "Dtm")
    );
}

// The values P130DateFilters.al:255-293 pins stay clean passes.
#[test]
fn the_pinned_insert_values_stay_clean() {
    assert_eq!(
        unflagged("Insert", vec![date(1753, 1, 1, false), BLANK_DTM]),
        bad("get", "01/01/1753", "Dt")
    );
    assert_eq!(
        unflagged("Insert", vec![BLANK_DT, datetime(1753, 1, 1, 0)]),
        bad("get", "01/01/1753 12:00 AM", "Dtm")
    );
}

// al-runner issue 8006: 1754-01-01 is the first date BC stores, as Date and as DateTime.
#[test]
fn the_first_day_of_1754_stores() {
    assert_eq!(
        unflagged("Insert", vec![date(1754, 1, 1, false), BLANK_DTM]),
        "stored|stored:1754-01-01||Dialog"
    );
    assert_eq!(
        unflagged("Insert", vec![BLANK_DT, datetime(1754, 1, 1, 1_000)]),
        "stored|stored:|1754-01-01T00:00:01Z|Dialog"
    );
    assert_eq!(
        unflagged("Modify", vec![date(1754, 1, 1, false), BLANK_DTM]),
        "stored|stored:1754-01-01||Dialog"
    );
}

// al-runner issue 8006: the Modify of a 1753 value fails with the same text.
#[test]
fn a_modify_of_a_1753_date_fails() {
    assert_eq!(
        flagged("Modify", vec![date(1753, 1, 2, false), BLANK_DTM]),
        bad("modify", "01/02/1753", "Dt")
    );
    assert_eq!(
        flagged("Modify", vec![date(1753, 1, 1, false), BLANK_DTM]),
        bad("modify", "01/01/1753", "Dt")
    );
    assert_eq!(
        flagged("Modify", vec![BLANK_DT, datetime(1753, 12, 31, 86_399_000)]),
        bad("modify", "12/31/1753 11:59 PM", "Dtm")
    );
    assert_eq!(
        flagged("Modify", vec![BLANK_DT, datetime(1753, 1, 1, 1_000)]),
        bad("modify", "01/01/1753 12:00 AM", "Dtm")
    );
}

// Not measured: a temporary record keeps main's result (it stores the value).
#[test]
fn a_temporary_record_keeps_the_value() {
    assert_eq!(
        unflagged("TempInsertModify", vec![date(1753, 1, 2, false)]),
        "1753-01-02"
    );
}

// Not measured, refused: a closing date in 1753, a DateTime whose hour shows with one digit (whether it is padded is not
// measured), a millisecond the column rounds into another minute, a Modify whose result is used or that runs table code, a
// Modify before 1753 and a Rename.
#[test]
fn the_unmeasured_neighbors_are_refused() {
    refused("Insert", vec![date(1753, 6, 1, true), BLANK_DTM]);
    refused("Insert", vec![BLANK_DT, datetime(1753, 6, 1, 3 * 3_600_000)]);
    refused("Insert", vec![BLANK_DT, datetime(1753, 6, 1, 13 * 3_600_000)]);
    refused(
        "Insert",
        vec![BLANK_DT, datetime(1753, 6, 1, 23 * 3_600_000 + 59 * 60_000 + 59_999)],
    );
    refused("ModifyUsed", vec![date(1753, 1, 2, false)]);
    refused("ModifyWithTrigger", vec![date(1753, 1, 2, false)]);
    refused("Modify", vec![date(1752, 12, 31, false), BLANK_DTM]);
    refused("Modify", vec![BLANK_DT, datetime(1752, 12, 31, 12 * 3_600_000)]);
    refused("RenameTo", vec![date(1753, 1, 2, false)]);
}
