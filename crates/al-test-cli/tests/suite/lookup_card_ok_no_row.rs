//! OK of a Card page variable in lookup mode whose view is filtered to no row: the action is not found
//! (pages/lt-y68-lookup-getrecord/Y68LookupGetRecord.al:425 Q3_TestPageHandler_OK_NoRow_Card, corpus master 0d01b0cc).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69960 "LC Row" { fields { field(1; No; Code[10]) { } } keys { key(PK; No) { } } }
page 69961 "LC Card"
{
    PageType = Card;
    SourceTable = "LC Row";
    layout { area(Content) { group(G) { field(No; Rec.No) { } } } }
}
codeunit 69962 T
{
    Subtype = Test;
    local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;
    [TryFunction]
    local procedure TryRun(var Row: Record "LC Row")
    var
        P: Page "LC Card";
    begin
        P.SetTableView(Row);
        P.LookupMode(true);
        P.RunModal();
    end;
    [Test]
    [HandlerFunctions('CardOk')]
    procedure OkOnNoRow()
    var
        Row: Record "LC Row";
    begin
        Row.No := 'A';
        Row.Insert();
        Row.SetRange(No, 'ZZ');
        if TryRun(Row) then
            Error('no error');
        Check('The built-in action = OK is not found on the page.', GetLastErrorText());
        Check('TestActionNotFound', GetLastErrorCode());
    end;
    [ModalPageHandler]
    procedure CardOk(var TP: TestPage "LC Card")
    begin
        TP.OK().Invoke();
    end;
}
"#;

#[test]
fn ok_of_a_lookup_card_on_no_row_is_not_found() {
    let out = outcomes_in(SRC, "T");
    assert_eq!(out, vec![("OkOnNoRow".to_string(), Outcome::Pass)]);
}
