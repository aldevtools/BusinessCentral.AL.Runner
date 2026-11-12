//! `TestPage.Close()` in the handler of a TableRelation lookup's list page (al-runner issue 7479): allowed from the places
//! pages/close-errors/CemLookupTests.al pins (a field of a top-level Card, a field of a ListPart on a Document; the corpus
//! cases are the tests of that), refused from every other place, here a ListPart on a ListPlus host, the reporter's shape
//! (crates/al-vm/src/pages/relation_lookup_close.rs `relation_origin`).

use al_testrun::Outcome;

const SRC: &str = r#"
table 69970 "RLC Code" { LookupPageId = "RLC Codes"; fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
table 69971 "RLC Head" { fields { field(1; No; Code[20]) { } } keys { key(PK; No) { } } }
table 69972 "RLC Line"
{
    fields { field(1; No; Code[20]) { } field(2; LineNo; Integer) { } field(3; Ref; Code[20]) { TableRelation = "RLC Code"; } }
    keys { key(PK; No, LineNo) { } }
}
page 69973 "RLC Codes"
{
    PageType = List; SourceTable = "RLC Code"; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } }
}
page 69974 "RLC Lines"
{
    PageType = ListPart; SourceTable = "RLC Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(LineNo; Rec.LineNo) { } field(Ref; Rec.Ref) { } } } }
}
page 69975 "RLC Head ListPlus"
{
    PageType = ListPlus; SourceTable = "RLC Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(No; Rec.No) { } part(Lines; "RLC Lines") { SubPageLink = No = field(No); } } }
}
codeunit 69976 "RLC Tests"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('CloseHandler')]
    procedure ListPlusPart_Lookup_HandlerClose()
    var H: Record "RLC Head"; L: Record "RLC Line"; C: Record "RLC Code"; P: TestPage "RLC Head ListPlus";
    begin
        C.Code := 'C1'; C.Insert(); H.No := 'H1'; H.Insert(); L.No := 'H1'; L.LineNo := 1; L.Insert();
        P.OpenEdit(); P.GoToRecord(H); P.Lines.First(); P.Lines.Ref.Lookup();
    end;
    [ModalPageHandler]
    procedure CloseHandler(var Codes: TestPage "RLC Codes")
    begin
        Codes.Close();
    end;
}
"#;

#[test]
fn close_in_the_handler_of_a_lookup_from_a_listplus_part_stays_refused() {
    let o = al_testkit::outcomes_in(SRC, "RLC Tests");
    let (_, outcome) = o
        .iter()
        .find(|(n, _)| n == "ListPlusPart_Lookup_HandlerClose")
        .expect("test");
    match outcome {
        Outcome::NotRun(why) => assert!(why.contains("TestPage.Close() in a ModalPageHandler"), "{why}"),
        other => panic!("{other:?}"),
    }
}
