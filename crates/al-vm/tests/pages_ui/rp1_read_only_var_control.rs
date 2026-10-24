//! A page whose property says `Editable = false`: every control reads Editable() = No, a control over a global variable
//! too, also one with its own `Editable = true` and one in a group outside the repeater; on a List and a Card, opened for
//! edit or for view (corpus pages/lt-outside-worksheet/RmReadOnlyPageVarControl.al:146-:277, al-runner issue 5755).
//! A page of another type is not pinned.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69990 "RV Row" { fields { field(1; Code; Code[10]) { } field(2; Name; Text[50]) { } } keys { key(PK; Code) { } } }
page 69991 "RV List"
{
    PageType = List; SourceTable = "RV Row"; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) {
        group(Header) { field(HeaderText; HeaderText) { } }
        repeater(R) {
            field(Name; Rec.Name) { }
            field(NameText; NameText) { }
            field(NameTextEditable; NameText2) { Editable = true; }
        } } }
    var HeaderText: Text; NameText: Text; NameText2: Text;
}
page 69992 "RV Card"
{
    PageType = Card; SourceTable = "RV Row"; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { group(G) { field(Name; Rec.Name) { } field(NameText; NameText) { } } } }
    var NameText: Text;
}
page 69993 "RV Worksheet"
{
    PageType = Worksheet; SourceTable = "RV Row"; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(NameText; NameText) { } } } }
    var NameText: Text;
}
codeunit 69994 C
{
    procedure ListEdit(): Text
    var R: Record "RV Row"; P: TestPage "RV List";
    begin
        R.Code := 'A'; R.Insert();
        P.OpenEdit();
        exit(Format(P.Editable()) + Format(P.Name.Editable()) + Format(P.NameText.Editable()) +
            Format(P.NameTextEditable.Editable()) + Format(P.HeaderText.Editable()));
    end;
    procedure CardView(): Text
    var R: Record "RV Row"; P: TestPage "RV Card";
    begin
        R.Code := 'A'; R.Insert();
        P.OpenView();
        exit(Format(P.NameText.Editable()));
    end;
    procedure Worksheet(): Text
    var R: Record "RV Row"; P: TestPage "RV Worksheet";
    begin
        R.Code := 'A'; R.Insert();
        P.OpenEdit();
        exit(Format(P.NameText.Editable()));
    end;
}
"#;

#[test]
fn every_control_of_a_read_only_list_reads_not_editable() {
    assert_eq!(text(run_in(SRC, "C", "ListEdit")), "NoNoNoNoNo");
}

#[test]
fn a_variable_control_of_a_read_only_card_opened_for_view_reads_not_editable() {
    assert_eq!(text(run_in(SRC, "C", "CardView")), "No");
}

#[test]
fn a_variable_control_of_a_read_only_worksheet_is_refused() {
    assert!(matches!(
        run_in(SRC, "C", "Worksheet"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}
