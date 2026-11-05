//! CurrPage.SetSelectionFilter in OnAfterGetCurrRecord of a ListPart whose SubPageLink selects no row (al-runner issue 6964; Base
//! Application's "Doc. Attachment List Factbox"): the part's Rec holds the linked fields and the selection is that record, a filter on
//! its key and a count of 0, whether the table has rows of other documents or none (corpus branch al-runtime-rs-staging-r6962,
//! pages/parts/SelectionFilterLinkedPartNoRow.al).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 50120 "SP Head" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 50121 "SP Line"
{
    fields { field(1; "Table ID"; Integer) { } field(2; "No."; Code[20]) { } field(3; "Line No."; Integer) { } }
    keys { key(PK; "Table ID", "No.", "Line No.") { Clustered = true; } }
}
codeunit 50124 "SP Log" { SingleInstance = true; var L: Text; procedure Add(E: Text) begin if L <> '' then L += ' | '; L += E; end;
    procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end; }
page 50122 "SP Factbox"
{
    PageType = ListPart; SourceTable = "SP Line";
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    trigger OnAfterGetCurrRecord()
    var Selected: Record "SP Line"; Log: Codeunit "SP Log";
    begin
        CurrPage.SetSelectionFilter(Selected);
        Log.Add('rec=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.") + ' count=' + Format(Selected.Count()) + ' filters=' + Selected.GetFilters());
    end;
}
page 50123 "SP Card"
{
    PageType = Card; SourceTable = "SP Head";
    layout
    {
        area(Content) { field(NoCtl; Rec."No.") { } }
        area(FactBoxes) { part(Attachments; "SP Factbox") { SubPageLink = "Table ID" = const(50120), "No." = field("No."); } }
    }
}
codeunit 50125 T
{
    Subtype = Test;
    local procedure Start()
    var Head: Record "SP Head"; Line: Record "SP Line";
    begin
        Head.DeleteAll(); Line.DeleteAll(); Head."No." := 'A'; Head.Insert();
    end;
    local procedure AddLine(No: Code[20]; LineNo: Integer)
    var Line: Record "SP Line";
    begin
        Line."Table ID" := 50120; Line."No." := No; Line."Line No." := LineNo; Line.Insert();
    end;
    [Test] procedure OtherRows()
    var Card: TestPage "SP Card"; Log: Codeunit "SP Log";
    begin
        Start(); AddLine('B', 1); AddLine('B', 2);
        Card.OpenView();
        Error('%1', Log.Take());
    end;
    [Test] procedure Empty()
    var Card: TestPage "SP Card"; Log: Codeunit "SP Log";
    begin
        Start();
        Card.OpenView();
        Error('%1', Log.Take());
    end;
    [Test] procedure OwnRow()
    var Card: TestPage "SP Card"; Log: Codeunit "SP Log";
    begin
        Start(); AddLine('A', 7); AddLine('B', 1);
        Card.OpenView();
        Error('%1', Log.Take());
    end;
}
"#;

/// SelectionFilterLinkedPartNoRow.al: `rec=95656/A/0 count=0 filters=Table ID: 95656, No.: A, Line No.: 0` (_TableHasOtherRows,
/// _TableEmpty), `rec=95656/A/7 count=1 filters=... Line No.: 7` (_OnOwnRow), here with table 50120.
#[test]
fn selection_of_a_linked_part_on_no_row_is_its_linked_record() {
    let got: Vec<_> = outcomes_in(SRC, "T").into_iter().map(|(_, o)| o).collect();
    let fail = |s: &str| Outcome::Fail(s.into());
    assert_eq!(
        got,
        vec![
            fail("rec=50120/A/0 count=0 filters=Table ID: 50120, No.: A, Line No.: 0"),
            fail("rec=50120/A/0 count=0 filters=Table ID: 50120, No.: A, Line No.: 0"),
            fail("rec=50120/A/7 count=1 filters=Table ID: 50120, No.: A, Line No.: 7"),
        ]
    );
}
