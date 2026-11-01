//! A page whose changed row is no longer stored under the key it has (corpus master 97c29f86, pages/lt-y17-save-row-gone/
//! Y17SaveRowGone.al). A: a StandardDialog in lookup mode over a temporary source whose field OnValidate changes the primary key
//! renames the row at LookupOK and LookupCancel: the table's OnRename runs, not OnModify (:459, :530). B: a card action hands Rec
//! to a codeunit that modifies and deletes it: the edit is saved before the action, the card closes with the last error code
//! PageCloseRequest and nothing is re-inserted (:632). A list in that state is refused (:711).

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 51540 "SR Profile"
{
    fields { field(1; Code; Code[20]) { } field(2; Mail; Boolean) { } field(3; Note; Text[30]) { } }
    keys { key(PK; Code) { } }
    trigger OnModify() begin L.Add('T.OnModify:' + Rec.Code); end;
    trigger OnRename() begin L.Add('T.OnRename:' + Rec.Code + ' x=' + xRec.Code); end;
    var L: Codeunit "SR Log";
}
codeunit 51541 "SR Log"
{
    SingleInstance = true;
    var E: Text; Cancel: Boolean;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
    procedure SetCancel(V: Boolean) begin Cancel := V; end;
    procedure GetCancel(): Boolean begin exit(Cancel); end;
}
page 51542 "SR Options"
{
    PageType = StandardDialog; SourceTable = "SR Profile"; SourceTableTemporary = true;
    layout
    {
        area(Content)
        {
            field(Mail; Rec.Mail) { trigger OnValidate() begin Rec.Code := 'CUSTOM'; end; }
            field(Note; Rec.Note) { }
        }
    }
    var L: Codeunit "SR Log";
    trigger OnOpenPage() begin CurrPage.LookupMode := true; end;
    trigger OnAfterGetCurrRecord() begin L.Add('P.AGCR:' + Rec.Code); end;
    trigger OnModifyRecord(): Boolean begin L.Add('P.OnModifyRecord:' + Rec.Code); exit(true); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin L.Add('P.OnQueryClosePage:' + Format(CloseAction)); exit(true); end;
}
table 51543 "SR Doc"
{
    fields { field(1; "No."; Code[20]) { } field(2; Name; Text[30]) { } field(3; Done; Boolean) { } }
    keys { key(PK; "No.") { } }
    trigger OnModify() begin L.Add('T.OnModify:' + Rec."No." + '/' + Rec.Name); end;
    var L: Codeunit "SR Log";
}
codeunit 51544 "SR Finish"
{
    TableNo = "SR Doc";
    trigger OnRun() begin Rec.Done := true; Rec.Modify(); Rec.Delete(); end;
}
page 51545 "SR Doc Card"
{
    PageType = Card; SourceTable = "SR Doc";
    layout { area(Content) { field("No."; Rec."No.") { } field(Name; Rec.Name) { } } }
    actions { area(Processing) { action(Finish) { trigger OnAction() begin L.Add('P.Finish'); Codeunit.Run(Codeunit::"SR Finish", Rec); end; } } }
    var L: Codeunit "SR Log";
    trigger OnAfterGetCurrRecord() begin L.Add('P.AGCR:' + Rec."No."); end;
    trigger OnModifyRecord(): Boolean begin L.Add('P.OnModifyRecord:' + Rec."No."); exit(true); end;
}
codeunit 51546 "SR Tests"
{
    Subtype = Test;
    local procedure Rename(Cancel: Boolean): Text
    var Temp: Record "SR Profile" temporary; L: Codeunit "SR Log"; Result: Action; s: Text;
    begin
        Temp.Code := 'DEF'; Temp.Note := 'temp'; Temp.Insert();
        L.SetCancel(Cancel); L.Take();
        Result := Page.RunModal(Page::"SR Options", Temp);
        s := Format(Result) + '|' + L.Take() + '|' + Temp.Code + '/' + Format(Temp.Mail) + '/' + Temp.Note + '|' + Format(Temp.Count());
        exit(s + '|' + Format(Temp.Get('DEF')));
    end;
    [Test]
    [HandlerFunctions('OptionsHandler')]
    procedure RenameOK()
    begin
        Check(Rename(false), 'LookupOK|P.AGCR:DEF;T.OnRename:CUSTOM x=DEF;P.OnQueryClosePage:LookupOK;|CUSTOM/Yes/temp|1|No');
    end;
    [Test]
    [HandlerFunctions('OptionsHandler')]
    procedure RenameCancel()
    begin
        Check(Rename(true), 'LookupCancel|P.AGCR:DEF;T.OnRename:CUSTOM x=DEF;P.AGCR:CUSTOM;P.OnQueryClosePage:LookupCancel;|CUSTOM/Yes/temp|1|No');
    end;
    [Test]
    procedure CardDeleted()
    begin
        Check(CardDeletedText(), 'P.OnModifyRecord:A;T.OnModify:A/edited;P.Finish;|PageCloseRequest||B/b;');
    end;
    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('got %1', Actual);
    end;
    [ModalPageHandler]
    procedure OptionsHandler(var P: TestPage "SR Options")
    var L: Codeunit "SR Log";
    begin
        P.Mail.SetValue(true);
        if L.GetCancel() then P.Cancel().Invoke() else P.OK().Invoke();
    end;
    local procedure CardDeletedText(): Text
    var D: Record "SR Doc"; P: TestPage "SR Doc Card"; L: Codeunit "SR Log"; s: Text;
    begin
        D.DeleteAll();
        D."No." := 'A'; D.Name := 'a'; D.Insert(); D."No." := 'B'; D.Name := 'b'; D.Insert();
        P.OpenEdit(); P.GoToKey('A'); P.Name.SetValue('edited'); L.Take();
        ClearLastError();
        P.Finish.Invoke();
        s := L.Take() + '|' + GetLastErrorCode() + '|' + GetLastErrorText() + '|';
        if D.FindSet() then repeat s += D."No." + '/' + D.Name + ';'; until D.Next() = 0;
        exit(s);
    end;
}
"#;

#[test]
fn a_changed_key_is_renamed_at_lookup_ok_and_cancel_and_a_card_closes_on_its_deleted_row() {
    let o = outcomes_in(SRC, "SR Tests");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    assert_eq!(get("RenameOK"), "Some(Pass)");
    assert_eq!(get("RenameCancel"), "Some(Pass)");
    assert_eq!(get("CardDeleted"), "Some(Pass)");
}
