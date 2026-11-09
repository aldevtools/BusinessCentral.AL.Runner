//! The CloseAction and the Rec that OnQueryClosePage of a Card sees when an action deleted the card's row and BC closes the
//! page by force (corpus pages/deleted-rows-navigation/Y89GoneSingleMove.al, codeunit 94730, the Y89_CA_True_* tests; the
//! pinned values below are their Expect lines).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 51620 "Y9 Row" { fields { field(1; Id; Integer) { } field(2; Txt; Text[30]) { } } keys { key(PK; Id) { } } }
codeunit 51621 "Y9 Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin if E <> '' then E += ';'; E += T; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51622 "Y9 Card" { PageType = Card; SourceTable = "Y9 Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } field(TxtCtl; Rec.Txt) { } } }
    actions { area(Processing) {
        action(DelAll) { trigger OnAction() var R: Record "Y9 Row"; begin R.DeleteAll(true); end; }
        action(DelAllUpd) { trigger OnAction() var R: Record "Y9 Row"; begin R.DeleteAll(true); CurrPage.Update(false); end; }
        action(DelCur) { trigger OnAction() begin Rec.Delete(true); end; } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin L.Add('Q:' + Format(CloseAction) + ':' + Format(Rec.Id)); exit(true); end;
    trigger OnClosePage() begin L.Add('X'); end;
    var L: Codeunit "Y9 Log"; }
page 51623 "Y9 CardGet" { PageType = Card; SourceTable = "Y9 Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } } }
    actions { area(Processing) { action(DelCur) { trigger OnAction() begin Rec.Delete(true); end; } } }
    trigger OnAfterGetRecord() begin L.Add('G:' + Format(Rec.Id)); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin L.Add('Q:' + Format(CloseAction) + ':' + Format(Rec.Id)); exit(true); end;
    trigger OnClosePage() begin L.Add('X'); end;
    var L: Codeunit "Y9 Log"; }
codeunit 51625 C
{
    local procedure Seed(Count: Integer)
    var R: Record "Y9 Row"; I: Integer;
    begin
        R.DeleteAll();
        for I := 1 to Count do begin R.Id := I * 10; R.Insert(); end;
    end;
    local procedure Cell(Count: Integer; View: Boolean; Which: Integer): Text
    var L: Codeunit "Y9 Log"; P: TestPage "Y9 Card";
    begin
        Seed(Count); L.Take();
        if View then P.OpenView() else P.OpenEdit();
        case Which of
            1: P.DelCur.Invoke();
            2: P.DelAllUpd.Invoke();
            3: P.DelAll.Invoke();
        end;
        exit(L.Take());
    end;
    local procedure LastCell(View: Boolean): Text
    var L: Codeunit "Y9 Log"; P: TestPage "Y9 Card";
    begin
        Seed(3);
        if View then P.OpenView() else P.OpenEdit();
        P.GoToKey(30);
        L.Take();
        P.DelCur.Invoke();
        exit(L.Take());
    end;
    procedure LastRow(): Text
    begin
        exit(LastCell(false) + '|' + LastCell(true));
    end;
    procedure LastRowGet(): Text
    var L: Codeunit "Y9 Log"; P: TestPage "Y9 CardGet";
    begin
        Seed(3);
        P.OpenEdit();
        P.GoToKey(30);
        L.Take();
        P.DelCur.Invoke();
        exit(L.Take());
    end;
    procedure Matrix(): Text
    begin
        exit(Cell(1, false, 1) + '|' + Cell(1, false, 2) + '|' + Cell(1, true, 3) + '|' + Cell(2, false, 1) + '|' + Cell(2, true, 1) + '|'
            + Cell(2, false, 2) + '|' + Cell(2, true, 3));
    end;
}
"#;

/// Y89_CA_True_{OpenEdit,OpenView}_1Row_{delcur,delallupd,delall} `action log` `D:10/Q:OK:0;X`; _2Row_delcur `D:10/Q:OK:20;X`;
/// _2Row_{delallupd,delall} `D:10;D:20/Q:OK:0;X`: the action is OK, Rec stands on the next row, or is blank when none is left.
#[test]
fn a_forced_close_of_a_card_passes_ok_and_the_next_row() {
    assert_eq!(
        text(run_in(SRC, "C", "Matrix")),
        "Q:OK:0;X|Q:OK:0;X|Q:OK:0;X|Q:OK:20;X|Q:OK:20;X|Q:OK:0;X|Q:OK:0;X"
    );
}

// A card whose deleted row was the last row of its view, with rows before it: not pinned by the corpus. Microsoft's tests
// (Tests-ERM-Purchase, codeunit 134099 PostPurchaseInvoiceCardWithBlankQuantityIsFoundationFALSE and others) post the last
// purchase document from its card and need the forced close to finish. Assumed (flagged): OnQueryClosePage gets the same
// action as when a row after it is left, and Rec stands on the nearest row before the deleted one, without OnAfterGetRecord.
#[test]
fn a_forced_close_of_a_card_whose_deleted_row_was_last_passes_ok_and_the_row_before() {
    use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
    clear_hits();
    assert_eq!(text(run_in(SRC, "C", "LastRow")), "Q:OK:20;X|Q:OK:20;X");
    assert!(take_hits().contains(&Assumption::GoneCardLastRowClosesOnRowBefore));
    // `G:30` is the read of the row as the action left it, before the page finds it gone (as before this assumption); no read
    // of row 20 follows, before or after the close.
    assert_eq!(text(run_in(SRC, "C", "LastRowGet")), "G:30;Q:OK:20;X");
}
