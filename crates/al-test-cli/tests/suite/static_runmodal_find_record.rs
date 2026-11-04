//! The static `Page.RunModal(0, Rec)` and `Page.RunModal(Page::"..", Rec)` of a list page with OnFindRecord (al-runner issue 6992;
//! Base Application's "Item Variants"): the page opens in lookup mode and asks `=>`, as LookupMode(true) of a Page variable
//! (corpus branch al-runtime-rs-staging-r6962, pages/run-modal/StaticRunModalFindRecordLookup.al).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 50760 "SR Row" { LookupPageId = "SR List"; fields { field(1; Code; Code[10]) { } field(2; Grp; Code[10]) { } } keys { key(PK; Code) { } } }
codeunit 50762 "SR Log" { SingleInstance = true; var L: Text; procedure Add(E: Text) begin L += E + ';'; end; procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end; }
page 50761 "SR List"
{
    PageType = List; SourceTable = "SR Row"; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(CodeCtl; Rec.Code) { } } } }
    trigger OnFindRecord(Which: Text): Boolean var Log: Codeunit "SR Log"; Found: Boolean;
    begin Found := Rec.Find(Which); Log.Add('F:' + Which + '=' + Format(Found)); exit(Found); end;
    trigger OnNextRecord(Steps: Integer): Integer var Log: Codeunit "SR Log"; Moved: Integer;
    begin Moved := Rec.Next(Steps); Log.Add('N:' + Format(Steps) + '=' + Format(Moved)); exit(Moved); end;
    trigger OnAfterGetRecord() var Log: Codeunit "SR Log"; begin Log.Add('R:' + Rec.Code); end;
}
codeunit 50763 T
{
    Subtype = Test;
    var Log: Codeunit "SR Log";
    local procedure Start(var Row: Record "SR Row"; Filtered: Boolean)
    begin
        Row.DeleteAll();
        AddRow('V1', 'X'); AddRow('V2', 'G'); AddRow('V3', 'G'); AddRow('V4', 'G');
        if Filtered then begin Row.FilterGroup(2); Row.SetRange(Grp, 'G'); Row.FilterGroup(0); end;
    end;
    local procedure AddRow(C: Code[10]; G: Code[10]) var Row: Record "SR Row"; begin Row.Code := C; Row.Grp := G; Row.Insert(); end;
    [Test] [HandlerFunctions('FirstOk')]
    procedure ZeroFiltered() var Row: Record "SR Row"; A: Text;
    begin Start(Row, true); A := Format(Page.RunModal(0, Row)); Error('%1 %2 %3', A, Log.Take(), Row.Code); end;
    [Test] [HandlerFunctions('FirstOk')]
    procedure PageIdFiltered() var Row: Record "SR Row"; A: Text;
    begin Start(Row, true); A := Format(Page.RunModal(Page::"SR List", Row)); Error('%1 %2 %3', A, Log.Take(), Row.Code); end;
    [Test] [HandlerFunctions('FirstOk')]
    procedure ZeroOnRow() var Row: Record "SR Row"; A: Text;
    begin Start(Row, true); Row.Get('V3'); A := Format(Page.RunModal(0, Row)); Error('%1 %2 %3', A, Log.Take(), Row.Code); end;
    [Test] [HandlerFunctions('FirstOk')]
    procedure ZeroNoFilter() var Row: Record "SR Row"; A: Text;
    begin Start(Row, false); A := Format(Page.RunModal(0, Row)); Error('%1 %2 %3', A, Log.Take(), Row.Code); end;
    [Test] [HandlerFunctions('CancelH')]
    procedure ZeroCancel() var Row: Record "SR Row"; A: Text;
    begin Start(Row, true); A := Format(Page.RunModal(0, Row)); Error('%1 %2 %3', A, Log.Take(), Row.Code); end;
    [ModalPageHandler] procedure FirstOk(var P: TestPage "SR List")
    begin Log.Add('H:opened=' + P.CodeCtl.Value()); P.First(); Log.Add('H:first=' + P.CodeCtl.Value()); P.OK().Invoke(); Log.Add('H:ok'); end;
    [ModalPageHandler] procedure CancelH(var P: TestPage "SR List")
    begin Log.Add('H:opened=' + P.CodeCtl.Value()); P.Cancel().Invoke(); Log.Add('H:cancel'); end;
}
"#;

/// StaticRunModalFindRecordLookup.al: the logs and rows BC 29 gave.
#[test]
fn static_runmodal_of_a_list_with_onfindrecord_asks_find_forward() {
    let got: Vec<_> = outcomes_in(SRC, "T").into_iter().map(|(_, o)| o).collect();
    let fail = |s: &str| Outcome::Fail(s.into());
    assert_eq!(
        got,
        vec![
            fail("LookupOK F:=>=Yes;R:V2;N:1=1;R:V3;N:1=1;R:V4;N:1=0;R:V2;N:-1=0;H:opened=V2;H:first=V2;H:ok; V2"),
            fail("LookupOK F:=>=Yes;R:V2;N:1=1;R:V3;N:1=1;R:V4;N:1=0;R:V2;N:-1=0;H:opened=V2;H:first=V2;H:ok; V2"),
            fail("LookupOK F:=>=Yes;R:V3;N:1=1;R:V4;N:1=0;R:V3;N:-1=-1;R:V2;N:-1=0;H:opened=V3;H:first=V2;H:ok; V2"),
            fail(
                "LookupOK F:=>=Yes;R:V1;N:1=1;R:V2;N:1=1;R:V3;N:1=1;R:V4;N:1=0;R:V1;N:-1=0;H:opened=V1;H:first=V1;H:ok; V1"
            ),
            fail(
                "LookupCancel F:=>=Yes;R:V2;N:1=1;R:V3;N:1=1;R:V4;N:1=0;R:V2;N:-1=0;H:opened=V2;F:==Yes;R:V2;H:cancel; V2"
            ),
        ]
    );
}
