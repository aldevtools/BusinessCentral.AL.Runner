//! A page extension that moves a field to the front of its repeater, a part to the front of the FactBoxes area, or a group to
//! the end of the Content area: the page opens and a TestPage reads the moved control (al-runner issue 6952). Pinned by
//! corpus pages/extension-layout-containers/TestRepeaterMove.Codeunit.al, TestFactboxMove.Codeunit.al and
//! TestContentMove.Codeunit.al (`LayoutMove_OpenPage_ReadsSeededValue`, passed on BC 27.5, 28.5 and 29.0).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

fn outcome(page: &str, ext: &str) -> Outcome {
    let src = format!(
        r#"
table 62960 "LM Row" {{ fields {{ field(1; Code; Code[20]) {{ }} field(2; Description; Text[50]) {{ }} }} keys {{ key(PK; Code) {{ Clustered = true; }} }} }}
page 62961 "LM Factbox" {{
    PageType = CardPart; SourceTable = "LM Row";
    layout {{ area(Content) {{ group(Data) {{ field(Description; Rec.Description) {{ }} }} }} }}
}}
page 62962 "LM Page" {{
    SourceTable = "LM Row";
    {page}
}}
pageextension 62963 "LM Ext" extends "LM Page" {{ layout {{ {ext} }} }}
codeunit 62964 T
{{
    Subtype = Test;
    [Test] procedure Run()
    var R: Record "LM Row"; P: TestPage "LM Page";
    begin
        R.Code := 'ROW1'; R.Description := 'Non-default layout value'; R.Insert();
        P.OpenView();
        P.Description.AssertEquals('Non-default layout value');
        P.Close();
    end;
}}"#
    );
    outcomes_in(&src, "T")[0].1.clone()
}

#[test]
fn a_field_moved_to_the_front_of_its_repeater() {
    let page = "PageType = List; layout { area(Content) { repeater(General) { field(Code; Rec.Code) { } field(Description; Rec.Description) { } } } }";
    assert_eq!(outcome(page, "movefirst(General; Description)"), Outcome::Pass);
}

#[test]
fn a_part_moved_to_the_front_of_the_factboxes() {
    let page = r#"PageType = Card; layout { area(Content) { group(Data) { field(Code; Rec.Code) { } field(Description; Rec.Description) { } } }
        area(FactBoxes) { part(Details; "LM Factbox") { SubPageLink = Code = field(Code); } } }"#;
    assert_eq!(outcome(page, "movefirst(FactBoxes; Details)"), Outcome::Pass);
}

#[test]
fn a_group_moved_to_the_end_of_the_content_area() {
    let page = "PageType = Card; layout { area(Content) { group(Data) { field(Code; Rec.Code) { } field(Description; Rec.Description) { } } } }";
    assert_eq!(outcome(page, "movelast(Content; Data)"), Outcome::Pass);
}
