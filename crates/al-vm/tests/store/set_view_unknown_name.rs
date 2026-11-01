//! SetView with a WHERE condition on a name the table has no field of drops that condition, and a SORTING of one such name
//! gives the primary key (record/lt-setview-caption-form/SetViewCaptionForm.al:179-191; types/lt-evaluate-format-lasterror-setview/
//! P86SetView.al:184, :222). `Field0` (the timestamp column) and lists that mix a known and an unknown name stay refused.

use al_builtins::native::Signal;
use al_testkit::text;

fn run(view: &str) -> Result<al_builtins::value::Value, Signal> {
    let src = format!(
        r#"
table 69980 "SVU Row" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Code[10]) {{ }} }} keys {{ key(PK; Id) {{ }} key(N; Name) {{ }} }} }}
codeunit 69981 C
{{
    procedure F(): Text
    var A: Record "SVU Row";
    begin
        A.Id := 1; A.Insert();
        A.Id := 2; A.Insert();
        A.SetCurrentKey(Name);
        A.SetRange(Id, 1);
        A.SetView('{view}');
        exit(Format(A.HasFilter()) + '|' + Format(A.Count()) + '|' + A.GetView(false));
    end;
}}
"#
    );
    al_testkit::run_in(&src, "C", "F")
}

#[test]
fn an_unknown_name_in_where_is_dropped() {
    assert_eq!(
        text(run("WHERE(Nothing Here=CONST(5))")),
        "No|2|VERSION(1) SORTING(Field1)"
    );
    assert_eq!(text(run("WHERE(Type=FILTER(Item))")), "No|2|VERSION(1) SORTING(Field1)");
}

#[test]
fn an_unknown_name_in_sorting_gives_the_primary_key() {
    assert_eq!(text(run("SORTING(Nonexistent)")), "No|2|VERSION(1) SORTING(Field1)");
}

#[test]
fn field_zero_and_mixed_lists_stay_refused() {
    for view in [
        "WHERE(Field0=FILTER(A))",
        "WHERE(Nope=CONST(1),Id=CONST(1))",
        "SORTING(Name,Nonexistent)",
    ] {
        assert!(matches!(run(view), Err(Signal::Unsupported(_))), "{view}");
    }
}

#[test]
fn an_empty_sorting_is_an_invalid_view() {
    // record/lt-setview-sorting-nonkey/SetViewSortingNonKey.al:205, :224-226 `empty_sorting`.
    assert_eq!(
        al_testkit::error_message(run("SORTING()")),
        "Invalid expression of type: table view.\\SORTING()"
    );
}
