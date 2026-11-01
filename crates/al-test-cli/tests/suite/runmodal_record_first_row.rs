//! A list run with the static `Page.RunModal(id, Rec)` on a record that names the first row of an unfiltered view reads its
//! rows as a list opened on its first row does (the window, the row once more, then OnAfterGetCurrRecord), and a GoToKey in
//! the handler reads the new row twice before OnAfterGetCurrRecord, as the same page run from a page variable with no record
//! (pages/lt-ai-list-order/LookupGoToKeyTypedReads.al:204-221 Y7_PageRunModalRecord_GoToKey_TypedReads_OK against :163
//! Y7_NotLookupMode_GoToKey_TypedReads_OK: `open=R:A,R:B,R:C,R:A,R:A,C:A;goto=Yes;after goto=R:B,R:B,C:B`).

use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 69980 "RF Row" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { Clustered = true; } } }
page 69981 "RF List"
{
    PageType = List; SourceTable = "RF Row"; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Lines) { field("Code"; Rec."Code") { } } } }
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."Code"); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Rec."Code"); end;
    var Log: Codeunit "RF Log";
}
codeunit 69982 "RF Log"
{
    SingleInstance = true;
    var Lines: Text;
    procedure Add(Line: Text) begin if Lines <> '' then Lines += ','; Lines += Line; end;
    procedure Take(): Text var R: Text; begin R := Lines; Lines := ''; exit(R); end;
}
codeunit 69983 T
{
    Subtype = Test;
    var Log: Codeunit "RF Log"; Seen: Text;
    local procedure Setup()
    var Row: Record "RF Row";
    begin
        Row.DeleteAll();
        Row."Code" := 'A'; Row.Insert(); Row."Code" := 'B'; Row.Insert(); Row."Code" := 'C'; Row.Insert();
        Log.Take(); Seen := '';
    end;
    [ModalPageHandler]
    procedure H(var L: TestPage "RF List")
    begin
        Seen += 'open=' + Log.Take() + ';';
        Seen += 'goto=' + Format(L.GoToKey('B')) + ';';
        Seen += 'after goto=' + Log.Take() + ';';
        L.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure PageVar()
    var P: Page "RF List";
    begin
        Setup();
        P.RunModal();
        if Seen <> 'open=R:A,R:B,R:C,R:A,R:A,C:A;goto=Yes;after goto=R:B,R:B,C:B;' then Error('%1', Seen);
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure StaticWithRecord()
    var Row: Record "RF Row";
    begin
        Setup();
        Row.Get('A');
        Page.RunModal(Page::"RF List", Row);
        if Seen <> 'open=R:A,R:B,R:C,R:A,R:A,C:A;goto=Yes;after goto=R:B,R:B,C:B;' then Error('%1', Seen);
    end;
}
"#;

#[test]
fn a_static_runmodal_on_the_first_row_reads_as_a_list_opened_on_its_first_row() {
    let out = al_testkit::outcomes_in(SRC, "T");
    assert_eq!(out.len(), 2);
    for (n, o) in out {
        assert_eq!(o, Outcome::Pass, "{n}");
    }
}
