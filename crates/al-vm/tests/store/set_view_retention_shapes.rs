//! SetView texts of the Retention Policy code (gap G5-14): a `field<n>` in any case, a `Field<n>` with no such field, and an
//! empty value in `1()` / `FILTER()` / `CONST()` (record/lt-y51-setview-shapes/Y51SetViewShapes.al).

use al_builtins::native::Signal;
use al_testkit::text;

const TABLE: &str = r#"
table 69990 "SVR Row" { fields { field(1; "Entry No."; Integer) { } field(2; "Date Field"; Date) { } field(4; Description; Text[100]) { } field(5; "Description 2"; Text[100]) { } } keys { key(PK; "Entry No.") { } } }
"#;

fn run(view: &str) -> Result<al_builtins::value::Value, Signal> {
    let src = format!(
        r#"{TABLE}
codeunit 69991 C
{{
    procedure F(): Text
    var R: Record "SVR Row"; RR: RecordRef; Ids: Text;
    begin
        R."Entry No." := 1; R."Date Field" := DMY2Date(1, 1, 2024); R.Description := 'A'; R.Insert();
        R.Init(); R."Entry No." := 2; R."Description 2" := 'D'; R.Insert();
        R.Init(); R."Entry No." := 3; R."Date Field" := DMY2Date(1, 3, 2024); R.Description := 'B'; R."Description 2" := 'E'; R.Insert();
        RR.Open(69990);
        RR.SetView('{view}');
        if RR.FindSet() then
            repeat
                Ids += Format(RR.Field(1).Value) + ',';
            until RR.Next() = 0;
        exit(RR.GetView(false) + '|' + RR.GetView(true) + '|' + RR.GetFilters() + '|' + Format(RR.Count()) + '|' + Ids);
    end;
}}
"#
    );
    al_testkit::run_in(&src, "C", "F")
}

#[test]
fn a_field_number_form_reads_in_any_case() {
    // Y51SetViewShapes.al:118-131 (SetView_LowercaseFieldN).
    assert_eq!(
        text(run("sorting (field2) where(field2=1(<>''''))")),
        "VERSION(1) SORTING(Field2) WHERE(Field2=1(<>''))|VERSION(1) SORTING(Date Field) WHERE(Date Field=FILTER(<>''))|Date Field: <>''|2|1,3,"
    );
    assert_eq!(
        text(run("WHERE(fIeLd4=1(A))")),
        "VERSION(1) SORTING(Field1) WHERE(Field4=1(A))|VERSION(1) SORTING(Entry No.) WHERE(Description=FILTER(A))|Description: A|1|1,"
    );
    assert_eq!(
        text(run("SORTING (Field2)")),
        "VERSION(1) SORTING(Field2)|VERSION(1) SORTING(Date Field)||3|2,1,3,"
    );
}

#[test]
fn an_empty_one_or_filter_value_is_a_filter_that_keeps_every_row() {
    // Y51SetViewShapes.al:141-154 (SetView_EmptyFilterValue).
    assert_eq!(
        text(run("WHERE(Field4=1())")),
        "VERSION(1) SORTING(Field1) WHERE(Field4=1(==))|VERSION(1) SORTING(Entry No.) WHERE(Description=FILTER(==))|Description: ==|3|1,2,3,"
    );
    assert_eq!(
        text(run("WHERE(Field4=FILTER())")),
        "VERSION(1) SORTING(Field1) WHERE(Field4=1(==))|VERSION(1) SORTING(Entry No.) WHERE(Description=FILTER(==))|Description: ==|3|1,2,3,"
    );
    assert_eq!(
        text(run("WHERE(Field2=1())")),
        "VERSION(1) SORTING(Field1) WHERE(Field2=1(==))|VERSION(1) SORTING(Entry No.) WHERE(Date Field=FILTER(==))|Date Field: ==|3|1,2,3,"
    );
    assert_eq!(
        text(run("WHERE(Field1=1())")),
        "VERSION(1) SORTING(Field1) WHERE(Field1=1(==))|VERSION(1) SORTING(Entry No.) WHERE(Entry No.=FILTER(==))|Entry No.: ==|3|1,2,3,"
    );
}

#[test]
fn an_empty_const_on_text_selects_the_blank_value() {
    // Y51SetViewShapes.al:144, :151 `text_const`.
    assert_eq!(
        text(run("WHERE(Field4=CONST())")),
        "VERSION(1) SORTING(Field1) WHERE(Field4=1(''))|VERSION(1) SORTING(Entry No.) WHERE(Description=FILTER(''))|Description: ''|1|2,"
    );
}

#[test]
fn a_field_number_with_no_such_field_is_ignored() {
    // Y51SetViewShapes.al:166-167, :171-172 (`field99_x`, `sorting_field99`).
    let none = "VERSION(1) SORTING(Field1)|VERSION(1) SORTING(Entry No.)||3|1,2,3,";
    assert_eq!(text(run("WHERE(Field99=1(x))")), none);
    assert_eq!(text(run("SORTING(Field99)")), none);
}

#[test]
fn field_zero_stays_refused() {
    // Y51SetViewShapes.al:164-173: Field0 is the timestamp column, which the runtime does not model as a field.
    assert!(matches!(run("WHERE(Field0=1())"), Err(Signal::Unsupported(_))));
}

#[test]
fn a_double_equals_value_is_the_filter_getview_writes_for_an_empty_value() {
    // Y51SetViewShapes.al:269-295 (SetView_DoubleEqualsValue): `1(==)` and `FILTER(==)` on a Text, Date or Integer field
    // filter nothing and read back as `==`; with a second field the other filter still applies (:298-308).
    let all = |n: &str, cap: &str| {
        format!("VERSION(1) SORTING(Field1) WHERE({n}=1(==))|VERSION(1) SORTING(Entry No.) WHERE({cap}=FILTER(==))|{cap}: ==|3|1,2,3,")
    };
    assert_eq!(text(run("WHERE(Field4=1(==))")), all("Field4", "Description"));
    assert_eq!(text(run("VERSION(1) SORTING(Field1) WHERE(Field4=1(==))")), all("Field4", "Description"));
    assert_eq!(
        text(run("VERSION(1) SORTING(Entry No.) WHERE(Description=FILTER(==))")),
        all("Field4", "Description")
    );
    assert_eq!(text(run("WHERE(Field5=1(==))")), all("Field5", "Description 2"));
    assert_eq!(text(run("WHERE(Field2=1(==))")), all("Field2", "Date Field"));
    assert_eq!(text(run("WHERE(Field1=1(==))")), all("Field1", "Entry No."));
    assert_eq!(
        text(run("VERSION(1) SORTING(Field1) WHERE(Field4=1(==),Field5=1(E))")),
        "VERSION(1) SORTING(Field1) WHERE(Field4=1(==),Field5=1(E))|VERSION(1) SORTING(Entry No.) WHERE(Description=FILTER(==),Description 2=FILTER(E))|Description: ==, Description 2: E|1|3,"
    );
}

#[test]
fn a_double_equals_const_is_the_literal_text() {
    // Y51SetViewShapes.al:283, :292 `text_const`.
    assert_eq!(
        text(run("WHERE(Field4=CONST(==))")),
        "VERSION(1) SORTING(Field1) WHERE(Field4=1('=='))|VERSION(1) SORTING(Entry No.) WHERE(Description=FILTER('=='))|Description: '=='|0|"
    );
}

#[test]
fn setfilter_double_equals_on_text_keeps_no_filter() {
    // Y51SetViewShapes.al:343, :351 `setfilter_eqeq`: SetFilter(Description, '==') leaves no filter (GetFilter '', no WHERE, 3 rows).
    let src = format!(
        r#"{TABLE}
codeunit 69992 D
{{
    procedure F(): Text
    var R: Record "SVR Row";
    begin
        R."Entry No." := 1; R.Insert(); R."Entry No." := 2; R.Insert(); R."Entry No." := 3; R.Insert();
        R.SetFilter(Description, 'x');
        R.SetFilter(Description, '==');
        exit(R.GetFilter(Description) + '|' + R.GetView(false) + '|' + Format(R.Count()));
    end;
}}
"#
    );
    assert_eq!(text(al_testkit::run_in(&src, "D", "F")), "|VERSION(1) SORTING(Field1)|3");
}
