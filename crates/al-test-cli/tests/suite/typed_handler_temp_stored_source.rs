//! A Page-typed PageHandler or ModalPageHandler for `Page.Run(Id, TempRec)` / `Page.RunModal(Id, TempRec)` on a list whose source
//! table is not temporary (al-runner issue 7018): the page's Rec becomes temporary and shares the caller's rows, filters and current
//! row; the stored rows stay out (corpus branch al-runtime-rs-staging-r6962, ui-handlers/TypedHandlerTempRecordStoredSource.al).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 50750 "TS Row" { fields { field(1; Id; Integer) { } field(2; Msg; Text[30]) { } } keys { key(PK; Id) { } } }
page 50751 "TS List"
{
    PageType = List; SourceTable = "TS Row"; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } }
    procedure RowCount(): Integer begin exit(Rec.Count()); end;
    procedure CurId(): Integer begin exit(Rec.Id); end;
    procedure IsTemp(): Boolean begin exit(Rec.IsTemporary()); end;
}
codeunit 50753 T
{
    Subtype = Test;
    var Seen: Text;
    local procedure Fill(var Temp: Record "TS Row" temporary)
    var Stored: Record "TS Row";
    begin
        Stored.Id := 9; Stored.Msg := 'db'; Stored.Insert();
        Temp.Id := 1; Temp.Msg := 'a'; Temp.Insert();
        Temp.Id := 2; Temp.Msg := 'b'; Temp.Insert();
        Temp.Id := 3; Temp.Msg := 'c'; Temp.Insert();
    end;
    local procedure After(var Temp: Record "TS Row" temporary)
    var Stored: Record "TS Row";
    begin
        Error('%1|%2|%3|%4', Seen, Temp.Id, Temp.Count(), Stored.Count());
    end;
    [Test] [HandlerFunctions('H')]
    procedure Filtered()
    var Temp: Record "TS Row" temporary;
    begin
        Fill(Temp); Temp.SetRange(Id, 2, 3); Temp.FindFirst();
        Page.Run(Page::"TS List", Temp);
        After(Temp);
    end;
    [Test] [HandlerFunctions('H')]
    procedure OnRowTwo()
    var Temp: Record "TS Row" temporary;
    begin
        Fill(Temp); Temp.Get(2);
        Page.Run(Page::"TS List", Temp);
        After(Temp);
    end;
    [Test] [HandlerFunctions('H')]
    procedure Empty()
    var Temp: Record "TS Row" temporary; Stored: Record "TS Row";
    begin
        Stored.Id := 9; Stored.Insert();
        Page.Run(Page::"TS List", Temp);
        After(Temp);
    end;
    [Test] [HandlerFunctions('M')]
    procedure Modal()
    var Temp: Record "TS Row" temporary; Reply: Text;
    begin
        Fill(Temp); Temp.Get(2);
        Reply := Format(Page.RunModal(Page::"TS List", Temp));
        Seen := Reply + ' ' + Seen;
        After(Temp);
    end;
    local procedure Describe(var Pg: Page "TS List"): Text
    var Got: Record "TS Row" temporary;
    begin
        Pg.GetRecord(Got);
        exit('got=' + Format(Got.Id) + '/' + Got.Msg + ' pagecount=' + Format(Pg.RowCount()) + ' cur=' + Format(Pg.CurId()) + ' temp=' + Format(Pg.IsTemp()));
    end;
    [PageHandler] procedure H(var Pg: Page "TS List") begin Seen := Describe(Pg); end;
    [ModalPageHandler] procedure M(var Pg: Page "TS List"; var Reply: Action) begin Seen := Describe(Pg); Reply := Action::LookupOK; end;
}
"#;

/// TypedHandlerTempRecordStoredSource.al: `got=2/b pagecount=2 cur=2 temp=Yes`, caller `2`/`2`, stored `1` (_Filtered);
/// `pagecount=3` on row 2 (_OnRowTwo); `got=0/ pagecount=0 cur=0 temp=Yes` (_Empty); LookupOK for RunModal.
#[test]
fn page_rec_becomes_temporary_and_shares_the_callers_rows() {
    let got: Vec<_> = outcomes_in(SRC, "T").into_iter().map(|(_, o)| o).collect();
    let fail = |s: &str| Outcome::Fail(s.into());
    assert_eq!(
        got,
        vec![
            fail("got=2/b pagecount=2 cur=2 temp=Yes|2|2|1"),
            fail("got=2/b pagecount=3 cur=2 temp=Yes|2|3|1"),
            fail("got=0/ pagecount=0 cur=0 temp=Yes|0|0|1"),
            fail("LookupOK got=2/b pagecount=3 cur=2 temp=Yes|2|3|1"),
        ]
    );
}
