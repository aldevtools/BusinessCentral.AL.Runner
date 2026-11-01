//! An OnModifyRecordEvent subscriber that answers AllowModify false: real BC runs it once for an action's SaveRecord and the
//! Close after it (handlers/TestPageTriggerEvents.al:181-206). What the page does with the vetoed edit is not pinned, so a
//! second save of it is refused.

const SRC: &str = r#"
table 50719 VRow { fields { field(1; Code; Code[10]) { } field(2; Val; Text[20]) { } } keys { key(PK; Code) { } } }
codeunit 50775 VLog { SingleInstance = true; var N: Integer; procedure Add() begin N += 1; end; procedure Count(): Integer begin exit(N); end; }
page 50776 VRows
{
    PageType = List; SourceTable = VRow; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } field(Val; Rec.Val) { } } } }
    actions { area(Processing) { action(DirectSave) { trigger OnAction() begin Rec.Validate(Val, 'DIRECT'); CurrPage.SaveRecord(); end; } } }
}
codeunit 50777 VSub
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Page, Page::VRows, 'OnModifyRecordEvent', '', false, false)]
    local procedure Veto(var Rec: Record VRow; var xRec: Record VRow; var AllowModify: Boolean)
    var L: Codeunit VLog;
    begin
        AllowModify := false; L.Add();
    end;
}
codeunit 50778 C {
    procedure SaveThenClose(): Integer
    var Row: Record VRow; P: TestPage VRows; S: Codeunit VSub; L: Codeunit VLog;
    begin
        Row.Code := 'L1'; Row.Val := 'BEFORE'; Row.Insert();
        BindSubscription(S);
        P.OpenEdit(); P.GoToKey('L1');
        P.DirectSave.Invoke();
        P.Close();
        exit(L.Count());
    end;
}
"#;

#[test]
fn a_second_save_of_an_edit_a_subscriber_vetoed_is_refused() {
    match al_testkit::run_in(SRC, "C", "SaveThenClose") {
        Err(al_builtins::native::Signal::Unsupported(r)) => assert!(r.contains("vetoed"), "{r}"),
        other => panic!("{other:?}"),
    }
}
