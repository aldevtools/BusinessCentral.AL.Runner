//! A control whose source is a Record method without parentheses (`field(Total; Rec.Count)`) binds as a field named Count and
//! is refused ("control Total is bound to unknown field Count"). No corpus case pins a Record method without parentheses as a
//! control source other than a table's own procedure (pages/record-method-controls/TestRecordMethodControls.al:39), so
//! `Rec.Count` stays refused, as on main, until a probe measures it (issue 7875).

use al_builtins::native::Signal;

const SRC: &str = r#"
table 69960 "RC Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
page 69961 "RC Host"
{
    PageType = List;
    SourceTable = "RC Row";
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Total; Rec.Count) { } } } }
}
codeunit 69962 C
{
    procedure Run()
    var
        Row: Record "RC Row";
        View: TestPage "RC Host";
    begin
        Row.Id := 1;
        Row.Insert(false);
        View.OpenView();
    end;
}
"#;

#[test]
fn rec_count_as_a_control_source_stays_refused_as_an_unknown_field() {
    match al_testkit::run_in(SRC, "C", "Run") {
        Err(Signal::Unsupported(why)) => {
            assert!(why.contains("control Total is bound to unknown field Count"), "{why}")
        }
        other => panic!("expected the refusal, got {other:?}"),
    }
}
