//! A Worksheet without the new-row line run modally whose action deletes its only row (corpus pages/lt-outside-worksheet/
//! RmWorksheetGoneRow.al:397 RM_Ws_RunModal_DeleteAllUpdate, al-runner issue 5752): First right after the open reads nothing,
//! the action reads the gone row, then OnNewRecord and OnAfterGetCurrRecord run once on a blank Rec, First answers No, and the
//! handler that returns leaves RunModal = LookupCancel.

use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 64700 "RW Line" { fields { field(1; "Line No."; Integer) { } } keys { key(PK; "Line No.") { } } }
codeunit 64701 "RW Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(S: Text) begin E += S + ';'; end;
    procedure Take(): Text var R: Text; begin R := E; E := ''; exit(R); end;
}
page 64702 "RW Ws"
{
    PageType = Worksheet; SourceTable = "RW Line"; InsertAllowed = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(LineNo; Rec."Line No.") { } } } }
    actions { area(Processing) { action(Gone) { trigger OnAction() var L: Record "RW Line"; begin L.DeleteAll(); CurrPage.Update(false); end; } } }
    trigger OnAfterGetRecord() var L: Codeunit "RW Log"; begin L.Add('R:' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "RW Log"; begin L.Add('C:' + Format(Rec."Line No.")); end;
    trigger OnNewRecord(BelowxRec: Boolean) var L: Codeunit "RW Log"; begin L.Add('N:' + Format(Rec."Line No.")); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean var L: Codeunit "RW Log"; begin L.Add('Q'); exit(true); end;
    trigger OnClosePage() var L: Codeunit "RW Log"; begin L.Add('X'); end;
}
codeunit 64703 T
{
    Subtype = Test;
    var Got: Text;

    [Test]
    [HandlerFunctions('H')]
    procedure RunModal()
    var R: Record "RW Line"; L: Codeunit "RW Log"; A: Action;
    begin
        R."Line No." := 10000; R.Insert();
        L.Take();
        A := Page.RunModal(Page::"RW Ws");
        Got += '|' + L.Take() + '|' + Format(A);
        if Got <> 'Yes/R:10000;R:10000;C:10000;/R:10000;N:0;C:0;/No|Q;X;|LookupCancel' then
            Error(Got);
    end;

    [ModalPageHandler]
    procedure H(var P: TestPage "RW Ws")
    var L: Codeunit "RW Log";
    begin
        Got := Format(P.First()) + '/' + L.Take();
        P.Gone.Invoke();
        Got += '/' + L.Take() + '/' + Format(P.First());
    end;
}
"#;

#[test]
fn a_worksheet_run_modally_that_lost_its_only_row_shows_a_blank_rec() {
    let o = al_testkit::outcomes_in(SRC, "T");
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, Outcome::Pass, "{}", o[0].0);
}
