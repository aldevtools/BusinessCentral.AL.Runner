//! Page.GetRecord after a TestPage handler cancelled a lookup list that showed no row: the record holds the value the view
//! filters the primary key to (pages/lt-y68-lookup-getrecord/Y68LookupGetRecord.al:446
//! Q3_TestPageHandler_Cancel_NoRow_NonEditableList, corpus master 0d01b0cc).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69965 "LK Row" { fields { field(1; No; Code[10]) { } field(2; Name; Text[30]) { } } keys { key(PK; No) { } } }
page 69966 "LK List"
{
    PageType = List;
    SourceTable = "LK Row";
    Editable = false;
    layout { area(Content) { repeater(R) { field(No; Rec.No) { } field(Name; Rec.Name) { } } } }
}
codeunit 69967 T
{
    Subtype = Test;
    local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;
    [Test]
    [HandlerFunctions('ListCancel')]
    procedure CancelOnNoRow()
    var
        Row: Record "LK Row";
        Cur: Record "LK Row";
        P: Page "LK List";
    begin
        Row.No := 'A';
        Row.Name := 'alpha';
        Row.Insert();
        Row.SetRange(No, 'ZZ');
        P.SetTableView(Row);
        P.LookupMode(true);
        Check('LookupCancel', Format(P.RunModal()));
        P.GetRecord(Cur);
        Check('ZZ||No', Cur.No + '|' + Cur.Name + '|' + Format(Cur.Find()));
    end;
    [ModalPageHandler]
    procedure ListCancel(var TP: TestPage "LK List")
    begin
        TP.Cancel().Invoke();
    end;
}
"#;

#[test]
fn getrecord_after_a_cancelled_lookup_on_no_row_holds_the_key_filter_value() {
    let out = outcomes_in(SRC, "T");
    assert_eq!(out, vec![("CancelOnNoRow".to_string(), Outcome::Pass)]);
}
