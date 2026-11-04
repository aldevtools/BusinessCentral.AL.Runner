//! A Page-typed PageHandler for a page run from a Page variable that its own procedure set up (Rec.Copy(Temp, true): the Error
//! Messages shape), and for Page.Run(Id, TempRec) with a temporary record (corpus
//! ui-handlers/Y72PageHandlerSetUpVariable.al).

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50740 "TH Row" { fields { field(1; Id; Integer) { } field(2; Msg; Text[30]) { } } keys { key(PK; Id) { } } }
page 50741 "TH Temp List"
{
    PageType = List; SourceTable = "TH Row"; SourceTableTemporary = true; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } }
    var Tag: Text;
    procedure SetRecords(var Temp: Record "TH Row" temporary) begin if Temp.FindFirst() then; Rec.Copy(Temp, true); end;
    procedure SetTag(NewTag: Text) begin Tag := NewTag; end;
    procedure GetTag(): Text begin exit(Tag); end;
    procedure RowCount(): Integer begin exit(Rec.Count()); end;
    procedure CurId(): Integer begin exit(Rec.Id); end;
    trigger OnOpenPage() begin Error('open ran'); end;
}
codeunit 50742 T
{
    Subtype = Test;
    var Seen: Text;
    local procedure Fill(var Temp: Record "TH Row" temporary)
    begin
        Temp.Id := 1; Temp.Msg := 'a'; Temp.Insert();
        Temp.Id := 2; Temp.Msg := 'b'; Temp.Insert();
        Temp.Id := 3; Temp.Msg := 'c'; Temp.Insert();
    end;
    [Test] [HandlerFunctions('H')]
    procedure SetByProcedure()
    var Temp: Record "TH Row" temporary; Pe: Page "TH Temp List";
    begin
        Fill(Temp);
        Pe.SetRecords(Temp);
        Pe.SetTag('t1');
        Pe.Run();
        if Seen + '|' + Format(Temp.Id) + '|' + Pe.GetTag() <> 'tag=t1 got=1/a pagecount=3 cur=1|1|' then
            Error('%1', Seen + '|' + Format(Temp.Id) + '|' + Pe.GetTag());
    end;
    [Test] [HandlerFunctions('H')]
    procedure TempRecordFiltered()
    var Temp: Record "TH Row" temporary;
    begin
        Fill(Temp);
        Temp.SetRange(Id, 2, 3);
        Temp.FindFirst();
        Page.Run(Page::"TH Temp List", Temp);
        if Seen <> 'tag= got=2/b pagecount=2 cur=2' then
            Error('%1', Seen);
    end;
    [PageHandler]
    procedure H(var Pg: Page "TH Temp List")
    var Got: Record "TH Row" temporary;
    begin
        Pg.GetRecord(Got);
        Seen := 'tag=' + Pg.GetTag() + ' got=' + Format(Got.Id) + '/' + Got.Msg + ' pagecount=' + Format(Pg.RowCount()) + ' cur=' + Format(Pg.CurId());
    end;
}
"#;

#[test]
fn a_page_typed_handler_sees_the_set_up_variable_and_a_shared_temporary_record() {
    // Y72PageHandlerSetUpVariable.al TypedHandler_SetByProcedure_TempShared, TypedHandler_TagOnly,
    // TypedHandler_TempRecord_PageRun_Filtered.
    let o = outcomes_in(SRC, "T");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    assert_eq!(get("SetByProcedure"), "Some(Pass)");
    assert_eq!(get("TempRecordFiltered"), "Some(Pass)");
}
