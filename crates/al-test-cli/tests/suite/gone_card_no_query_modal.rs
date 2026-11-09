//! A card without OnQueryClosePage run by Page.RunModal whose row the handler's action deletes: the card moves to the next row,
//! else the row before, runs OnAfterGetRecord for it, and OnClosePage sees Rec on it (blank when no row is left); RunModal
//! returns LookupCancel (corpus pages/deleted-rows-navigation/Y89GoneSingleMove.al, codeunit 94730, Y89_NQ_Modal_On20_delcur
//! `D:20/R:20;R:20;R:30;X:30`, _On30_delcur `D:30/R:30;R:30;R:20;X:20`, _Only_delcur `D:10/R:10;R:10;X:0`, _On20_delall
//! `D:10;D:20;D:30/R:20;R:20;X:0`, `result` LookupCancel in each).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 51640 "Y9Q Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 51641 "Y9Q Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin if E <> '' then E += ';'; E += T; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51642 "Y9Q Card" { PageType = Card; SourceTable = "Y9Q Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } } }
    actions { area(Processing) {
        action(DelAll) { trigger OnAction() var R: Record "Y9Q Row"; begin R.DeleteAll(true); end; }
        action(DelCur) { trigger OnAction() begin Rec.Delete(true); end; } } }
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec.Id)); end;
    trigger OnClosePage() begin L.Add('X:' + Format(Rec.Id)); end;
    var L: Codeunit "Y9Q Log"; }
codeunit 51645 T
{
    Subtype = Test;
    local procedure Run(Count: Integer; At: Integer): Text
    var R: Record "Y9Q Row"; L: Codeunit "Y9Q Log"; I: Integer; Result: Action;
    begin
        R.DeleteAll();
        for I := 1 to Count do begin R.Id := I * 10; R.Insert(); end;
        R.Get(At);
        L.Take();
        Result := Page.RunModal(Page::"Y9Q Card", R);
        exit(Format(Result) + '|' + L.Take());
    end;
    local procedure Check(Got: Text; Want: Text) begin if Got <> Want then Error('%1 <> %2', Got, Want); end;
    [Test]
    [HandlerFunctions('DelCurModal')]
    procedure On20DelCur() begin Check(Run(3, 20), 'LookupCancel|R:20;R:20;R:30;X:30'); end;
    [Test]
    [HandlerFunctions('DelCurModal')]
    procedure On30DelCur() begin Check(Run(3, 30), 'LookupCancel|R:30;R:30;R:20;X:20'); end;
    [Test]
    [HandlerFunctions('DelCurModal')]
    procedure OnlyDelCur() begin Check(Run(1, 10), 'LookupCancel|R:10;R:10;X:0'); end;
    [Test]
    [HandlerFunctions('DelAllModal')]
    procedure On20DelAll() begin Check(Run(3, 20), 'LookupCancel|R:20;R:20;X:0'); end;
    [ModalPageHandler]
    procedure DelCurModal(var P: TestPage "Y9Q Card") begin P.DelCur.Invoke(); end;
    [ModalPageHandler]
    procedure DelAllModal(var P: TestPage "Y9Q Card") begin P.DelAll.Invoke(); end;
}
"#;

#[test]
fn a_forced_close_of_a_modal_card_without_on_query_close_page_moves_and_returns_lookup_cancel() {
    let out = outcomes_in(SRC, "T");
    assert_eq!(out.len(), 4, "{out:?}");
    for (name, o) in &out {
        assert_eq!(*o, Outcome::Pass, "{name}: {o:?}");
    }
}
