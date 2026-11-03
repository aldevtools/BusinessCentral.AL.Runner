//! A list that lost its only row to an action, opened for view or edit (corpus master pages/lt-y80-gone-rows/Y80GoneRows.al,
//! codeunit 94230: the Y80_Gone_* tests; the pinned values below are its Expect lines). Plain editable list, list with
//! InsertAllowed = false, with InsertAllowed and ModifyAllowed = false, and with Editable = false.

const SRC: &str = r#"
table 51600 "Y8 Row" { fields { field(1; Id; Integer) { } field(2; Txt; Text[30]) { } } keys { key(PK; Id) { } } }
codeunit 51601 "Y8 Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin if E <> '' then E += ';'; E += T; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51602 "Y8 All" { PageType = List; SourceTable = "Y8 Row"; Editable = true;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } field(TxtCtl; Rec.Txt) { } } } }
    actions { area(Processing) { action(DelAllUpd) { trigger OnAction() var R: Record "Y8 Row"; begin R.DeleteAll(true); CurrPage.Update(false); end; } } }
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Format(Rec.Id)); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('N:' + Format(Rec.Id)); end;
    var L: Codeunit "Y8 Log"; }
page 51603 "Y8 NoNew" { PageType = List; SourceTable = "Y8 Row"; InsertAllowed = false;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } field(TxtCtl; Rec.Txt) { } } } }
    actions { area(Processing) { action(DelAllUpd) { trigger OnAction() var R: Record "Y8 Row"; begin R.DeleteAll(true); CurrPage.Update(false); end; } } }
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Format(Rec.Id)); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('N:' + Format(Rec.Id)); end;
    var L: Codeunit "Y8 Log"; }
page 51604 "Y8 ReadOnly" { PageType = List; SourceTable = "Y8 Row"; Editable = false;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } field(TxtCtl; Rec.Txt) { } } } }
    actions { area(Processing) { action(DelAllUpd) { trigger OnAction() var R: Record "Y8 Row"; begin R.DeleteAll(true); CurrPage.Update(false); end; } } }
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Format(Rec.Id)); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('N:' + Format(Rec.Id)); end;
    var L: Codeunit "Y8 Log"; }
page 51606 "Y8 Card" { PageType = Card; SourceTable = "Y8 Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } field(TxtCtl; Rec.Txt) { } } }
    actions { area(Processing) { action(DelAllUpd) { trigger OnAction() var R: Record "Y8 Row"; begin R.DeleteAll(true); CurrPage.Update(false); end; } } }
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec.Id)); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin L.Add('Q'); exit(true); end;
    trigger OnClosePage() begin L.Add('X'); end;
    var L: Codeunit "Y8 Log"; }
page 51607 "Y8 CardReads" { PageType = Card; SourceTable = "Y8 Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } } }
    actions { area(Processing) { action(DelAllUpd) { trigger OnAction() var R: Record "Y8 Row"; begin R.DeleteAll(true); CurrPage.Update(false); end; } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin L.Add(Format(CloseAction)); exit(true); end;
    var L: Codeunit "Y8 Log"; }
codeunit 51605 C
{
    procedure CardReadsCloseAction(): Text
    var R: Record "Y8 Row"; P: TestPage "Y8 CardReads";
    begin
        R.Id := 10; R.Insert();
        P.OpenEdit();
        P.DelAllUpd.Invoke();
        exit('done');
    end;
    procedure CardEdit(): Text
    var R: Record "Y8 Row"; L: Codeunit "Y8 Log"; P: TestPage "Y8 Card"; s: Text;
    begin
        R.Id := 10; R.Insert(); L.Take();
        P.OpenEdit(); s := L.Take();
        P.DelAllUpd.Invoke();
        exit(s + '|' + L.Take() + '|' + GetLastErrorCode());
    end;
    procedure NoNewView(): Text
    var R: Record "Y8 Row"; L: Codeunit "Y8 Log"; P: TestPage "Y8 NoNew"; s: Text;
    begin
        R.Id := 10; R.Insert(); L.Take();
        P.OpenView(); s := L.Take();
        P.DelAllUpd.Invoke(); s += '|' + L.Take() + '|' + P.IdCtl.Value() + '|' + Format(P.First()) + '|' + L.Take();
        P.DelAllUpd.Invoke(); s += '|' + L.Take();
        P.Close();
        exit(s);
    end;
    procedure ReadOnlyEdit(): Text
    var R: Record "Y8 Row"; L: Codeunit "Y8 Log"; P: TestPage "Y8 ReadOnly"; s: Text;
    begin
        R.Id := 10; R.Insert(); L.Take();
        P.OpenEdit(); s := L.Take();
        P.DelAllUpd.Invoke(); s += '|' + L.Take() + '|' + P.IdCtl.Value() + '|' + Format(P.First()) + '|' + L.Take();
        P.DelAllUpd.Invoke(); s += '|' + L.Take();
        P.Close();
        exit(s);
    end;
    procedure AllEditFirst(): Text
    var R: Record "Y8 Row"; L: Codeunit "Y8 Log"; P: TestPage "Y8 All"; s: Text;
    begin
        R.Id := 10; R.Insert(); L.Take();
        P.OpenEdit();
        P.DelAllUpd.Invoke(); L.Take();
        s := Format(P.First()) + '|' + L.Take();
        exit(s);
    end;
    procedure AllEditNext(): Text
    var R: Record "Y8 Row"; L: Codeunit "Y8 Log"; P: TestPage "Y8 All"; s: Text;
    begin
        R.Id := 10; R.Insert(); L.Take();
        P.OpenEdit();
        P.DelAllUpd.Invoke(); L.Take();
        s := Format(P.Next()) + '|' + L.Take() + '|' + P.IdCtl.Value();
        exit(s);
    end;
    procedure AllEditLast(): Text
    var R: Record "Y8 Row"; L: Codeunit "Y8 Log"; P: TestPage "Y8 All"; s: Text;
    begin
        R.Id := 10; R.Insert(); L.Take();
        P.OpenEdit();
        P.DelAllUpd.Invoke(); L.Take();
        s := Format(P.Last()) + '|' + L.Take() + '|' + P.IdCtl.Value();
        exit(s);
    end;
    procedure AllEditPrevious(): Text
    var R: Record "Y8 Row"; L: Codeunit "Y8 Log"; P: TestPage "Y8 All"; s: Text;
    begin
        R.Id := 10; R.Insert(); L.Take();
        P.OpenEdit();
        P.DelAllUpd.Invoke(); L.Take();
        s := Format(P.Previous()) + '|' + L.Take() + '|' + P.IdCtl.Value();
        exit(s);
    end;
    procedure AllEdit(): Text
    var R: Record "Y8 Row"; L: Codeunit "Y8 Log"; P: TestPage "Y8 All"; s: Text;
    begin
        R.Id := 10; R.Insert(); L.Take();
        P.OpenEdit(); s := L.Take();
        P.DelAllUpd.Invoke(); s += '|' + L.Take() + '|' + P.IdCtl.Value();
        s += '|' + Format(P.First()) + Format(P.Next()) + Format(P.Last()) + Format(P.Previous()) + '|' + L.Take();
        P.DelAllUpd.Invoke(); s += '|' + L.Take();
        P.Close();
        exit(s);
    end;
}
"#;

// Y80_Gone_NoNew_OpenView_DeleteAllUpdate: action log `R:10;D:10` (here the table has no OnDelete: `R:10`), no move logs, the second
// action logs nothing.
#[test]
fn a_list_without_the_new_row_line_opened_for_view_raises_nothing_after_it_lost_its_only_row() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "NoNewView")),
        "R:10;R:10;C:10|R:10||No||"
    );
}

// Y80_Gone_ReadOnly_OpenEdit_DeleteAllUpdate: the same for an Editable = false list opened for edit.
#[test]
fn a_read_only_list_opened_for_edit_raises_nothing_after_it_lost_its_only_row() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ReadOnlyEdit")),
        "R:10;R:10;C:10|R:10||No||"
    );
}

// Y80_Gone_All_OpenEdit_DeleteAllUpdate: `action log R:10;D:10;N:0;C:0;N:0;C:0;N:0;C:0`; First raises nothing and answers No
// (pages/lt-f3-pages/F3ListTests.al:189). A single move after the action (pages/lt-y89-gone-single-move/Y89GoneSingleMove.al,
// Y89_All_OpenEdit_delallupd_{first,next,previous,last}): all answer No and the Id control reads 0; Next and Previous log nothing,
// Last raises the pair twice (`move log N:0;C:0;N:0;C:0`). Together they log what Y80_Gone_All_OpenEdit_DeleteAllUpdate pins.
#[test]
fn first_on_the_blank_line_of_a_list_that_lost_its_only_row_raises_nothing() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "AllEditFirst")), "No|");
}

#[test]
fn next_and_previous_on_that_blank_line_answer_no_and_raise_nothing() {
    for routine in ["AllEditNext", "AllEditPrevious"] {
        assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", routine)), "No||0", "{routine}");
    }
}

#[test]
fn last_on_that_blank_line_answers_no_and_raises_the_pair_twice() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "AllEditLast")),
        "No|N:0;C:0;N:0;C:0|0"
    );
}

#[test]
fn the_four_moves_in_a_row_on_that_blank_line_raise_the_pair_twice_in_all() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "AllEdit")),
        "R:10;R:10;C:10|R:10;N:0;C:0;N:0;C:0;N:0;C:0|0|NoNoNoNo|N:0;C:0;N:0;C:0|N:0;C:0;N:0;C:0"
    );
}

// Y80_Gone_Card_OpenEdit_DeleteAllUpdate: `open log R:10`, `action log R:10;D:10;Q;X` (OnQueryClosePage, then OnClosePage).
#[test]
fn a_card_whose_row_an_action_deleted_asks_on_query_close_page_before_it_closes() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "CardEdit")),
        "R:10|R:10;Q;X|PageCloseRequest"
    );
}

// The CloseAction a card's OnQueryClosePage receives when its deleted row closes it is not pinned: a trigger that reads it is refused.
#[test]
fn a_card_whose_on_query_close_page_reads_the_close_action_of_a_forced_close_is_refused() {
    let r = al_testkit::run_in(SRC, "C", "CardReadsCloseAction");
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(w)) if w.contains("CloseAction")),
        "{r:?}"
    );
}
