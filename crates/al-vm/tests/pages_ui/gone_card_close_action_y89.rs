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
    trigger OnClosePage() begin L.Add('X:' + Format(Rec.Id)); end;
    var L: Codeunit "Y9 Log"; }
page 51624 "Y9 CardLast" { PageType = Card; SourceTable = "Y9 Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } } }
    actions { area(Processing) { action(DelCur) { trigger OnAction() begin Rec.Delete(true); end; } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin L.Add('Q:' + Format(CloseAction) + ':' + Format(Rec.Id)); exit(true); end;
    trigger OnClosePage() begin L.Add('X:' + Format(Rec.Id)); end;
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
    var L: Codeunit "Y9 Log"; P: TestPage "Y9 CardLast";
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
    procedure GetLastAndFirst(): Text
    begin
        exit(RowGet(30) + '|' + RowGet(10));
    end;
    local procedure RowGet(Id: Integer): Text
    var L: Codeunit "Y9 Log"; P: TestPage "Y9 CardGet";
    begin
        Seed(3);
        P.OpenEdit();
        P.GoToKey(Id);
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

/// Y89_CA_True_{OpenEdit,OpenView}_3Row_delcur_last `action log` `D:30/Q:OK:20;X:20`: a card on the last of rows 10, 20, 30
/// whose action deletes it closes with OK, and Rec stands on the row before it in OnQueryClosePage and OnClosePage.
#[test]
fn a_forced_close_of_a_card_whose_deleted_row_was_last_passes_ok_and_the_row_before() {
    assert_eq!(text(run_in(SRC, "C", "LastRow")), "Q:OK:20;X:20|Q:OK:20;X:20");
}

/// Y89_CA_Get_OpenEdit_3Row_delcur_last `D:30/G:30;G:20;Q:OK:20;X:20` and _delcur_first `D:10/G:10;G:20;Q:OK:20;X:20`: after
/// the read of the deleted row as the action left it, OnAfterGetRecord runs for the row the card moves to (before or after
/// the deleted one), then OnQueryClosePage and OnClosePage; no read follows the close.
#[test]
fn a_forced_close_of_a_card_runs_on_after_get_record_for_the_row_it_moves_to() {
    assert_eq!(
        text(run_in(SRC, "C", "GetLastAndFirst")),
        "G:30;G:20;Q:OK:20;X:20|G:10;G:20;Q:OK:20;X:20"
    );
}

const SRC_NQ: &str = r#"
table 51630 "Y9N Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 51631 "Y9N Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin if E <> '' then E += ';'; E += T; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51632 "Y9N Card" { PageType = Card; SourceTable = "Y9N Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } } }
    actions { area(Processing) {
        action(DelAll) { trigger OnAction() var R: Record "Y9N Row"; begin R.DeleteAll(true); end; }
        action(DelCur) { trigger OnAction() begin Rec.Delete(true); end; } } }
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec.Id)); end;
    trigger OnClosePage() begin L.Add('X:' + Format(Rec.Id)); end;
    var L: Codeunit "Y9N Log"; }
codeunit 51633 C
{
    local procedure Seed(Count: Integer)
    var R: Record "Y9N Row"; I: Integer;
    begin
        R.DeleteAll();
        for I := 1 to Count do begin R.Id := I * 10; R.Insert(); end;
    end;
    local procedure Cell(Count: Integer; View: Boolean; At: Integer; W: Integer): Text
    var L: Codeunit "Y9N Log"; P: TestPage "Y9N Card";
    begin
        Seed(Count);
        if View then P.OpenView() else P.OpenEdit();
        if At <> 0 then P.GoToKey(At);
        L.Take();
        if W = 1 then P.DelCur.Invoke() else P.DelAll.Invoke();
        exit(L.Take());
    end;
    procedure Matrix(): Text
    begin
        exit(Cell(3, false, 20, 1) + '|' + Cell(3, false, 20, 2) + '|' + Cell(3, true, 30, 1) + '|' + Cell(3, true, 30, 2) + '|'
            + Cell(1, false, 0, 1) + '|' + Cell(1, true, 0, 2));
    end;
}
"#;

/// Y89_NQ_{OpenEdit,OpenView}_On20_delcur `action log` `D:20/R:20;R:30;X:30`, _On20_delall `R:20;X:0`, _On30_delcur
/// `R:30;R:20;X:20`, _On30_delall `R:30;X:0`, _Only_{delcur,delall} `R:10;X:0`: a card without OnQueryClosePage closed by its
/// deleted row moves to the next row, else the row before, and runs OnAfterGetRecord for it; with no row left Rec is blank.
#[test]
fn a_forced_close_of_a_card_without_on_query_close_page_moves_to_the_next_row() {
    assert_eq!(
        text(run_in(SRC_NQ, "C", "Matrix")),
        "R:20;R:30;X:30|R:20;X:0|R:30;R:20;X:20|R:30;X:0|R:10;X:0|R:10;X:0"
    );
}
