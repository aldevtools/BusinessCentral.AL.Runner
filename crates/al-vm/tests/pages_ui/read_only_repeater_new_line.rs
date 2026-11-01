//! A list whose repeater says `Editable`: the value decides whether the blank new-row line shows. A repeater with `Editable =
//! false`, or a page variable that is false, has no line (Next on the last row answers false and stays there), on a List, a
//! Worksheet and a ListPart in a Card; a page variable that is true keeps the line; OpenView has none; New() raises nothing
//! (corpus branch al-runtime-rs-staging-y65 eefaf01b, pages/lt-y65-readonly-repeater/Y65ReadOnlyRepeater.al:247-471, al-runner
//! issue 6674). Any other expression (a procedure call) is not pinned and stays refused.

use al_testkit::{run_in, text};

fn page(id: u32, name: &str, page_type: &str, repeater_editable: &str, extra: &str) -> String {
    format!(
        r#"
page {id} "{name}"
{{
    PageType = {page_type}; SourceTable = "RO Task"; ApplicationArea = All; UsageCategory = None;
    layout {{ area(Content) {{ repeater(Lines) {{ {repeater_editable}
        field("Entry No."; Rec."Entry No.") {{ }} field(Description; Rec.Description) {{ }} }} }} }}
    {extra}
}}"#
    )
}

fn walk(name: &str, page: &str) -> String {
    format!(
        r#"
    [TryFunction]
    local procedure TryNew{name}(var P: TestPage "{page}")
    begin
        P.New();
    end;
    procedure Walk{name}(Rows: Integer; View: Boolean): Text
    var P: TestPage "{page}"; R: Text;
    begin
        Seed(Rows);
        if View then P.OpenView() else P.OpenEdit();
        R := 'open=' + P.Description.Value();
        R += ' first=' + Format(P.First()) + ':' + P.Description.Value();
        R += ' next=' + Format(P.Next()) + ':' + P.Description.Value();
        R += ' next=' + Format(P.Next()) + ':' + P.Description.Value() + ':' + P."Entry No.".Value();
        R += ' next=' + Format(P.Next()) + ':' + P.Description.Value();
        R += ' last=' + Format(P.Last()) + ':' + P.Description.Value();
        R += ' prev=' + Format(P.Previous()) + ':' + P.Description.Value();
        R += ' editable=' + Format(P.Description.Editable()) + '/' + Format(P.Editable());
        ClearLastError();
        if TryNew{name}(P) then
            R += ' new=ok:' + P.Description.Value()
        else
            R += ' new=error:' + GetLastErrorText() + '|' + GetLastErrorCode();
        P.Close();
        exit(R + ' rows=' + Count());
    end;"#
    )
}

fn src() -> String {
    let var_page = |init: &str| {
        format!(
            "var RowsEditable: Boolean; trigger OnInit() begin RowsEditable := {init}; end; procedure IsEditable(): Boolean begin exit(RowsEditable); end;"
        )
    };
    let mut s = String::from(
        r#"
table 69970 "RO Task" { fields { field(1; "Entry No."; Integer) { } field(2; Description; Text[50]) { } } keys { key(PK; "Entry No.") { } } }
page 69977 "RO Host"
{
    PageType = Card; SourceTable = "RO Task"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("Entry No."; Rec."Entry No.") { } part(Tasks; "RO ListPart") { } } }
}"#,
    );
    s += &page(69971, "RO List", "List", "Editable = false;", "");
    s += &page(
        69972,
        "RO List VarFalse",
        "List",
        "Editable = RowsEditable;",
        &var_page("false"),
    );
    s += &page(
        69973,
        "RO List VarTrue",
        "List",
        "Editable = RowsEditable;",
        &var_page("true"),
    );
    s += &page(69974, "RO Worksheet", "Worksheet", "Editable = false;", "");
    s += &page(69975, "RO ListPart", "ListPart", "Editable = false;", "");
    s += &page(
        69976,
        "RO List Call",
        "List",
        "Editable = IsEditable();",
        &var_page("false"),
    );
    s += r#"
codeunit 69978 C
{
    local procedure Seed(Rows: Integer)
    var Task: Record "RO Task"; I: Integer;
    begin
        Task.DeleteAll();
        for I := 1 to Rows do begin
            Task."Entry No." := I;
            if I = 1 then Task.Description := 'First' else Task.Description := 'Second';
            Task.Insert();
        end;
    end;
    local procedure Count(): Text
    var Task: Record "RO Task";
    begin
        exit(Format(Task.Count()));
    end;
    procedure Part(): Text
    var Host: TestPage "RO Host"; R: Text;
    begin
        Seed(2);
        Host.OpenEdit();
        R := 'open=' + Host.Tasks.Description.Value();
        R += ' first=' + Format(Host.Tasks.First()) + ':' + Host.Tasks.Description.Value();
        R += ' next=' + Format(Host.Tasks.Next()) + ':' + Host.Tasks.Description.Value();
        R += ' next=' + Format(Host.Tasks.Next()) + ':' + Host.Tasks.Description.Value();
        R += ' last=' + Format(Host.Tasks.Last()) + ':' + Host.Tasks.Description.Value();
        R += ' editable=' + Format(Host.Tasks.Description.Editable()) + '/' + Format(Host.Tasks.Editable());
        Host.Close();
        exit(R);
    end;"#;
    s += &walk("List", "RO List");
    s += &walk("VarFalse", "RO List VarFalse");
    s += &walk("VarTrue", "RO List VarTrue");
    s += &walk("Worksheet", "RO Worksheet");
    s += &walk("Call", "RO List Call");
    s += "\n}\n";
    s
}

fn walk_of(name: &str, rows: i32, view: bool) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let mut b = al_testkit::Built::from_source(&src());
    let (o, r) = b.routine_in("C", &format!("Walk{name}"));
    b.call(
        o,
        r,
        vec![
            al_builtins::value::Value::Integer(rows),
            al_builtins::value::Value::Boolean(view),
        ],
    )
    .map(|(v, _)| v)
}

// Y65ReadOnlyRepeater.al:255-257 (List), :297-299 (variable false), :463 (Worksheet `view` Editable() Yes).
const NO_LINE_EDIT: &str = "open=First first=Yes:First next=Yes:Second next=No:Second:2 next=No:Second last=Yes:Second prev=Yes:First editable=No/Yes new=ok: rows=2";
const NO_LINE_VIEW: &str = "open=First first=Yes:First next=Yes:Second next=No:Second:2 next=No:Second last=Yes:Second prev=Yes:First editable=No/No new=ok: rows=2";
const NO_LINE_EMPTY: &str =
    "open= first=No: next=No: next=No:: next=No: last=No: prev=No: editable=No/Yes new=ok: rows=0";

#[test]
fn a_repeater_with_editable_false_has_no_new_row_line() {
    assert_eq!(text(walk_of("List", 2, false)), NO_LINE_EDIT);
    assert_eq!(text(walk_of("List", 2, true)), NO_LINE_VIEW);
    assert_eq!(text(walk_of("List", 0, false)), NO_LINE_EMPTY);
}

#[test]
fn a_repeater_whose_editable_variable_is_false_has_no_new_row_line() {
    assert_eq!(text(walk_of("VarFalse", 2, false)), NO_LINE_EDIT);
    assert_eq!(text(walk_of("VarFalse", 2, true)), NO_LINE_VIEW);
    assert_eq!(text(walk_of("VarFalse", 0, false)), NO_LINE_EMPTY);
}

#[test]
fn a_repeater_whose_editable_variable_is_true_keeps_the_new_row_line() {
    // Y65ReadOnlyRepeater.al:339-341.
    assert_eq!(
        text(walk_of("VarTrue", 2, false)),
        "open=First first=Yes:First next=Yes:Second next=Yes::0 next=No: last=Yes:Second prev=Yes:First editable=Yes/Yes new=ok: rows=2"
    );
    assert_eq!(text(walk_of("VarTrue", 2, true)), NO_LINE_VIEW);
    assert_eq!(
        text(walk_of("VarTrue", 0, false)),
        "open= first=No: next=No: next=No::0 next=No: last=No: prev=No: editable=Yes/Yes new=ok: rows=0"
    );
}

#[test]
fn a_worksheet_whose_repeater_says_editable_false_has_no_new_row_line() {
    // Y65ReadOnlyRepeater.al:422-424.
    assert_eq!(text(walk_of("Worksheet", 2, false)), NO_LINE_EDIT);
    // A Worksheet opened for view answers Editable() Yes (:423).
    assert_eq!(text(walk_of("Worksheet", 2, true)), NO_LINE_EDIT);
    assert_eq!(text(walk_of("Worksheet", 0, false)), NO_LINE_EMPTY);
}

#[test]
fn a_list_part_whose_repeater_says_editable_false_has_no_new_row_line() {
    // Y65ReadOnlyRepeater.al:443.
    assert_eq!(
        text(run_in(&src(), "C", "Part")),
        "open=First first=Yes:First next=Yes:Second next=No:Second last=Yes:Second editable=No/Yes"
    );
}

#[test]
fn a_repeater_editable_procedure_call_stays_refused() {
    assert!(matches!(
        walk_of("Call", 2, false),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}
