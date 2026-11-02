//! `FilterPageBuilder.GetView(Name, false)` writes a Date or DateTime filter value in the XML form of a view, as a
//! record's view does (events/lt-platform-triggers/PtObserve.al:594-600 `view|m0` = `WHERE(Field5=1(2020-01-01..))`).

use al_builtins::value::Value;

const SRC: &str = r#"
table 60010 "FPB Date Row"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Date Field"; Date) { }
        field(3; "DateTime Field"; DateTime) { }
        field(4; Description; Text[100]) { }
    }
    keys { key(PK; "Entry No.") { } }
}
codeunit 50110 C
{
    procedure Dates(): Text
    var
        B: FilterPageBuilder;
    begin
        B.AddTable('Rows', Database::"FPB Date Row");
        B.SetView('Rows', StrSubstNo('WHERE(Field2=1(..%1), Field4=1(some description))', DMY2Date(2, 11, 2026)));
        exit(B.GetView('Rows', false) + '|' + B.GetView('Rows', true));
    end;

    procedure Start(): Text
    var
        B: FilterPageBuilder;
    begin
        B.AddTable('Rows', Database::"FPB Date Row");
        B.SetView('Rows', 'WHERE("Date Field"=FILTER(010120..))');
        exit(B.GetView('Rows', false));
    end;

    procedure DateTimes(): Text
    var
        B: FilterPageBuilder;
    begin
        B.AddTable('Rows', Database::"FPB Date Row");
        B.SetView('Rows', StrSubstNo('WHERE(Field3=1(..%1))', Format(CreateDateTime(DMY2Date(2, 11, 2026), 120000T), 0, 1)));
        exit(B.GetView('Rows', false));
    end;
}
"#;

fn text(routine: &str) -> String {
    match al_testkit::run_in(SRC, "C", routine) {
        Ok(Value::Text(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_date_filter_is_written_in_the_xml_form_without_names() {
    assert_eq!(
        text("Start"),
        "VERSION(1) SORTING(Field1) WHERE(Field2=1(2020-01-01..))"
    );
    // System Application Test, Reten. Policy Setup Test.TestSetRetentionPolicyLineFilterView: the view set in the
    // session form reads back in the XML form; with names too.
    assert_eq!(
        text("Dates"),
        "VERSION(1) SORTING(Field1) WHERE(Field2=1(..2026-11-02),Field4=1(some description))|VERSION(1) SORTING(Entry No.) WHERE(Date Field=FILTER(..2026-11-02),Description=FILTER(some description))"
    );
}

#[test]
fn a_datetime_filter_is_written_in_utc_without_names() {
    assert_eq!(
        text("DateTimes"),
        "VERSION(1) SORTING(Field1) WHERE(Field3=1(..2026-11-02T12:00:00Z))"
    );
}
