//! A card run by Page.RunModal whose row the handler's action deletes: OnQueryClosePage gets LookupCancel and the next row
//! (or a blank Rec), then OnClosePage runs and RunModal returns LookupCancel (corpus pages/deleted-rows-navigation/
//! Y89GoneSingleMove.al, codeunit 94730, Y89_CA_Modal_True_1Row_DeleteCurrent `D:10/Q:LookupCancel:0;X`,
//! Y89_CA_Modal_True_2Row_DeleteCurrent `D:10/Q:LookupCancel:20;X`, result LookupCancel). The last of three rows (rows only
//! before it) is not pinned: assumption gone-card-last-row-closes-on-row-before, the row before (`LastOfThree`).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 51630 "Y9M Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 51631 "Y9M Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin if E <> '' then E += ';'; E += T; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51632 "Y9M Card" { PageType = Card; SourceTable = "Y9M Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } } }
    actions { area(Processing) { action(DelCur) { trigger OnAction() begin Rec.Delete(true); end; } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin L.Add('Q:' + Format(CloseAction) + ':' + Format(Rec.Id)); exit(true); end;
    trigger OnClosePage() begin L.Add('X'); end;
    var L: Codeunit "Y9M Log"; }
codeunit 51635 T
{
    Subtype = Test;
    local procedure Run(Count: Integer): Text
    begin
        exit(RunOn(Count, false));
    end;
    local procedure RunOn(Count: Integer; OnLast: Boolean): Text
    var R: Record "Y9M Row"; L: Codeunit "Y9M Log"; I: Integer; Result: Action;
    begin
        R.DeleteAll();
        for I := 1 to Count do begin R.Id := I * 10; R.Insert(); end;
        L.Take();
        if OnLast then R.FindLast() else R.FindFirst();
        Result := Page.RunModal(Page::"Y9M Card", R);
        exit(Format(Result) + '|' + L.Take());
    end;
    [Test]
    [HandlerFunctions('DelCurModal')]
    procedure OneRow()
    var S: Text;
    begin
        S := Run(1);
        if S <> 'LookupCancel|Q:LookupCancel:0;X' then Error('one row: %1', S);
    end;
    [Test]
    [HandlerFunctions('DelCurModal')]
    procedure TwoRows()
    var S: Text;
    begin
        S := Run(2);
        if S <> 'LookupCancel|Q:LookupCancel:20;X' then Error('two rows: %1', S);
    end;
    [Test]
    [HandlerFunctions('DelCurModal')]
    procedure LastOfThree()
    var S: Text;
    begin
        S := RunOn(3, true);
        if S <> 'LookupCancel|Q:LookupCancel:20;X' then Error('last of three: %1', S);
    end;
    [ModalPageHandler]
    procedure DelCurModal(var P: TestPage "Y9M Card")
    begin
        P.DelCur.Invoke();
    end;
}
"#;

#[test]
fn a_forced_close_of_a_modal_card_passes_lookup_cancel_and_the_next_row() {
    let out = outcomes_in(SRC, "T");
    for (name, o) in &out {
        assert_eq!(*o, Outcome::Pass, "{name}: {o:?}");
    }
    assert_eq!(out.len(), 3);
}
