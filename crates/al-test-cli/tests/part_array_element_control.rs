//! al-runner issue 6379: a ListPart control bound to an element of a page array variable (`ElementText[2]`) shows what
//! OnAfterGetRecord left in it for the row, as a control bound to a plain page variable does. Measured on real BC by corpus
//! writer Y5 (branch al-runtime-rs-staging-y5, pages/lt-y5/Y5PartArrayElementControl.al, codeunit 90983): `cell of A` after
//! First, `cell of B` after Next; an Integer element 10/20/30 for rows 1-3; an element never assigned gives ''; Last,
//! Previous, GoToKey and a read without First the same. AL source -> compile -> run as a test codeunit.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62540 "PAE Host" { fields { field(1; "Code"; Code[10]) { } } keys { key(PK; "Code") { } } }
table 62541 "PAE Row" { fields { field(1; "Row ID"; Integer) { } field(2; "No."; Code[20]) { } } keys { key(PK; "Row ID") { } } }
page 62542 "PAE Part" { PageType = ListPart; SourceTable = "PAE Row"; Editable = false;
    layout { area(Content) { repeater(Rows) {
        field("No."; Rec."No.") { }
        field(Plain; PlainText) { }
        field(Element; ElementText[2]) { }
        field(Never; ElementText[1]) { }
        field(Num; ElementNo[3]) { } } } }
    trigger OnAfterGetRecord() begin PlainText := 'cell of ' + Rec."No."; ElementText[2] := 'cell of ' + Rec."No."; ElementNo[3] := Rec."Row ID" * 10; end;
    var PlainText: Text; ElementText: array[3] of Text; ElementNo: array[3] of Integer; }
page 62543 "PAE Host" { PageType = Document; SourceTable = "PAE Host"; Editable = false;
    layout { area(Content) { group(General) { field("Code"; Rec."Code") { } } part(Cells; "PAE Part") { } } } }
codeunit 62544 "PAE Seed" {
    procedure Seed() var H: Record "PAE Host"; R: Record "PAE Row"; begin
        H.DeleteAll(); R.DeleteAll(); H.Code := 'H'; H.Insert();
        R."Row ID" := 1; R."No." := 'A'; R.Insert(); R."Row ID" := 2; R."No." := 'B'; R.Insert(); R."Row ID" := 3; R."No." := 'C'; R.Insert(); end; }
"#;

fn passes(tests: &str) {
    let src = format!("{OBJECTS}\ncodeunit 62550 T {{ Subtype = Test; {tests} }}");
    let o = al_testkit::outcomes_in(&src, "T");
    assert!(!o.is_empty());
    let bad: Vec<_> = o.into_iter().filter(|(_, r)| *r != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn an_array_element_control_of_a_list_part_shows_the_rows_value() {
    passes(
        r#"[Test] procedure FirstNext() var S: Codeunit "PAE Seed"; H: TestPage "PAE Host"; T: Text; begin
            S.Seed(); H.OpenView();
            H.Cells.First(); T := H.Cells.Element.Value() + ','; H.Cells.Next(); T += H.Cells.Element.Value() + ',';
            if T <> 'cell of A,cell of B,' then Error('element %1', T); end;
        [Test] procedure Plain() var S: Codeunit "PAE Seed"; H: TestPage "PAE Host"; T: Text; begin
            S.Seed(); H.OpenView();
            H.Cells.First(); T := H.Cells.Plain.Value() + ','; H.Cells.Next(); T += H.Cells.Plain.Value() + ',';
            if T <> 'cell of A,cell of B,' then Error('plain %1', T); end;
        [Test] procedure IntegerAndNever() var S: Codeunit "PAE Seed"; H: TestPage "PAE Host"; T: Text; begin
            S.Seed(); H.OpenView();
            H.Cells.First(); T := H.Cells.Num.Value(); H.Cells.Next(); T += '/' + H.Cells.Num.Value(); H.Cells.Next(); T += '/' + H.Cells.Num.Value();
            if T <> '10/20/30' then Error('num %1', T);
            if H.Cells.Never.Value() <> '' then Error('never %1', H.Cells.Never.Value()); end;
        [Test] procedure LastPreviousGoTo() var S: Codeunit "PAE Seed"; H: TestPage "PAE Host"; begin
            S.Seed(); H.OpenView();
            if H.Cells.Element.Value() <> 'cell of A' then Error('opening %1', H.Cells.Element.Value());
            H.Cells.Last(); if H.Cells.Element.Value() <> 'cell of C' then Error('last %1', H.Cells.Element.Value());
            H.Cells.Previous(); if H.Cells.Element.Value() <> 'cell of B' then Error('previous %1', H.Cells.Element.Value());
            H.Cells.GoToKey(1); if H.Cells.Element.Value() <> 'cell of A' then Error('goto %1', H.Cells.Element.Value()); end;"#,
    );
}
