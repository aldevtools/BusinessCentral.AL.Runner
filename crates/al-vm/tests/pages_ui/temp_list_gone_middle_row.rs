//! A read-only List over a temporary table whose action deletes the row it stands on, the middle one of three (al-runner issue
//! 7940; open corpus PR 1367, testpage/lifecycle/ReopenSameVariableState.al:469 TempListRowGone_CloseReopen_DeleteAgain): BC
//! lands on the row before it. The page copies the probe's; the neighbors flip one fact each.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 75370 "FL Row"
{
    fields { field(1; "Code"; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
    trigger OnModify() begin end;
}
table 75376 "FL Plain Row"
{
    fields { field(1; "Code"; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
    trigger OnModify() begin end;
}
page 75373 "FL Temp List"
{
    PageType = List; SourceTable = "FL Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } } }
    actions { area(Processing) { action(DelCur) { ApplicationArea = All; trigger OnAction() begin Log.Add('del(' + Rec."Code" + ')'); Rec.Delete(); end; } } }
    var Log: Codeunit "FL Log";
    trigger OnOpenPage()
    var Row: Record "FL Row";
    begin
        if Row.FindSet() then
            repeat
                Rec := Row;
                Rec.Insert();
            until Row.Next() = 0;
        Log.Add('open');
    end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Rec."Code" + ')'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
page 75377 "FL Temp Edit List"
{
    PageType = List; SourceTable = "FL Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } } }
    actions { area(Processing) { action(DelCur) { ApplicationArea = All; trigger OnAction() begin Log.Add('del(' + Rec."Code" + ')'); Rec.Delete(); end; } } }
    var Log: Codeunit "FL Log";
    trigger OnOpenPage()
    var Row: Record "FL Row";
    begin
        if Row.FindSet() then
            repeat
                Rec := Row;
                Rec.Insert();
            until Row.Next() = 0;
        Log.Add('open');
    end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Rec."Code" + ')'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
page 75378 "FL Stored List"
{
    PageType = List; SourceTable = "FL Plain Row"; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } } }
    actions { area(Processing) { action(DelCur) { ApplicationArea = All; trigger OnAction() begin Log.Add('del(' + Rec."Code" + ')'); Rec.Delete(); end; } } }
    var Log: Codeunit "FL Log";
    trigger OnOpenPage() begin Log.Add('open'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Rec."Code" + ')'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
page 75380 "FL Temp Update List"
{
    PageType = List; SourceTable = "FL Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } } }
    actions { area(Processing) { action(DelCur) { ApplicationArea = All; trigger OnAction() var Tmp: Record "FL Row" temporary; begin Log.Add('del(' + Rec."Code" + ')'); Rec.Delete(); CurrPage.Update(false); end; } } }
    var Log: Codeunit "FL Log";
    trigger OnOpenPage()
    var Row: Record "FL Row";
    begin
        if Row.FindSet() then
            repeat
                Rec := Row;
                Rec.Insert();
            until Row.Next() = 0;
        Log.Add('open');
    end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Rec."Code" + ')'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
page 75381 "FL Temp ModOther List"
{
    PageType = List; SourceTable = "FL Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } } }
    actions { area(Processing) { action(DelCur) { ApplicationArea = All; trigger OnAction() var Tmp: Record "FL Row" temporary; begin Log.Add('del(' + Rec."Code" + ')'); Rec.Delete(); Tmp.Copy(Rec, true); Tmp.Get('R1'); Tmp.Name := 'chg'; Tmp.Modify(); end; } } }
    var Log: Codeunit "FL Log";
    trigger OnOpenPage()
    var Row: Record "FL Row";
    begin
        if Row.FindSet() then
            repeat
                Rec := Row;
                Rec.Insert();
            until Row.Next() = 0;
        Log.Add('open');
    end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Rec."Code" + ')'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
page 75382 "FL Temp First List"
{
    PageType = List; SourceTable = "FL Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } } }
    actions { area(Processing) { action(DelCur) { ApplicationArea = All; trigger OnAction() var Tmp: Record "FL Row" temporary; begin Log.Add('del(' + Rec."Code" + ')'); Rec.Delete(); end; } } }
    var Log: Codeunit "FL Log";
    trigger OnOpenPage()
    var Row: Record "FL Row";
    begin
        if Row.FindSet() then
            repeat
                Rec := Row;
                Rec.Insert();
            until Row.Next() = 0;
        Rec.FindFirst();
        Log.Add('open');
    end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Rec."Code" + ')'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
page 75383 "FL Temp ArgCall List"
{
    PageType = List; SourceTable = "FL Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } } }
    actions { area(Processing) { action(DelCur) { ApplicationArea = All; trigger OnAction() var Tmp: Record "FL Row" temporary; begin Log.Add(Format(Tmp.IsEmpty())); Rec.Delete(); end; } } }
    var Log: Codeunit "FL Log";
    trigger OnOpenPage()
    var Row: Record "FL Row";
    begin
        if Row.FindSet() then
            repeat
                Rec := Row;
                Rec.Insert();
            until Row.Next() = 0;
        Log.Add('open');
    end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Rec."Code" + ')'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
page 75384 "FL Temp RunObj List"
{
    PageType = List; SourceTable = "FL Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } } }
    actions { area(Processing) { action(DelCur) { ApplicationArea = All; trigger OnAction() begin Log.Add('del(' + Rec."Code" + ')'); Rec.Delete(); end; } action(Go) { ApplicationArea = All; RunObject = page "FL Stored List"; } } }
    var Log: Codeunit "FL Log";
    trigger OnOpenPage()
    var Row: Record "FL Row";
    begin
        if Row.FindSet() then
            repeat
                Rec := Row;
                Rec.Insert();
            until Row.Next() = 0;
        Log.Add('open');
    end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Rec."Code" + ')'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
page 75385 "FL Temp Touch List"
{
    PageType = List; SourceTable = "FL Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } } }
    actions { area(Processing) { action(DelCur) { ApplicationArea = All; trigger OnAction() begin Log.Touch('del(' + Rec."Code" + ')'); Rec.Delete(); end; } } }
    var Log: Codeunit "FL Log";
    trigger OnOpenPage()
    var Row: Record "FL Row";
    begin
        if Row.FindSet() then
            repeat
                Rec := Row;
                Rec.Insert();
            until Row.Next() = 0;
        Log.Add('open');
    end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Rec."Code" + ')'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
codeunit 75374 "FL Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Reset() begin Entries := ''; end;
    procedure Add(Line: Text) begin if Entries <> '' then Entries += ' '; Entries += Line; end;
    procedure Take() Result: Text begin Result := '[' + Entries + ']'; Entries := ''; end;
    procedure Touch(Line: Text)
    var Plain: Record "FL Plain Row";
    begin
        Add(Line);
        Plain.Get('R1'); Plain.Name := 'cu'; Plain.Modify();
    end;
}
codeunit 75375 T
{
    var Log: Codeunit "FL Log";
    local procedure Rows(N: Integer)
    var Row: Record "FL Row"; Plain: Record "FL Plain Row"; I: Integer;
    begin
        Row.DeleteAll(); Plain.DeleteAll();
        for I := 1 to N do begin
            Row."Code" := 'R' + Format(I); Row.Insert();
            Plain."Code" := 'R' + Format(I); Plain.Insert();
        end;
        Log.Reset();
    end;
    procedure Probe(): Text
    var P: TestPage "FL Temp List"; Out: Text;
    begin
        Rows(3);
        P.OpenView(); P.GoToKey('R2'); Out := Log.Take();
        P.DelCur.Invoke(); Out += ' ' + P."Code".Value() + ' ' + Log.Take();
        P.Next(); Out += ' ' + P."Code".Value() + ' ' + Log.Take();
        P.Close(); Out += ' ' + Log.Take();
        P.OpenView(); Out += ' ' + P."Code".Value();
        P.GoToKey('R2'); Out += ' ' + Log.Take();
        P.DelCur.Invoke(); Out += ' ' + P."Code".Value() + ' ' + Log.Take();
        P.Next(); Out += ' ' + P."Code".Value() + ' ' + Log.Take();
        P.Close(); Out += ' ' + Log.Take();
        exit(Out);
    end;
    local procedure DeleteAt(N: Integer; At: Text)
    var P: TestPage "FL Temp List";
    begin
        Rows(N); P.OpenView(); P.GoToKey(At); P.DelCur.Invoke();
    end;
    procedure First() begin DeleteAt(3, 'R1'); end;
    procedure Last() begin DeleteAt(3, 'R3'); end;
    procedure FourRows() begin DeleteAt(4, 'R2'); end;
    procedure OpenEdit()
    var P: TestPage "FL Temp List";
    begin
        Rows(3); P.OpenEdit(); P.GoToKey('R2'); P.DelCur.Invoke();
    end;
    procedure EditableList()
    var P: TestPage "FL Temp Edit List";
    begin
        Rows(3); P.OpenView(); P.GoToKey('R2'); P.DelCur.Invoke();
    end;
    procedure NextBefore()
    var P: TestPage "FL Temp List";
    begin
        Rows(3); P.OpenView(); P.GoToKey('R1'); P.Next(); P.DelCur.Invoke();
    end;
    procedure UpdateInAction()
    var P: TestPage "FL Temp Update List";
    begin
        Rows(3); P.OpenView(); P.GoToKey('R2'); P.DelCur.Invoke();
    end;
    procedure ModifyOtherInAction()
    var P: TestPage "FL Temp ModOther List";
    begin
        Rows(3); P.OpenView(); P.GoToKey('R2'); P.DelCur.Invoke();
    end;
    procedure OpenOnFirst()
    var P: TestPage "FL Temp First List";
    begin
        Rows(3); P.OpenView(); P.GoToKey('R2'); P.DelCur.Invoke();
    end;
    procedure ArgCallInAction()
    var P: TestPage "FL Temp ArgCall List";
    begin
        Rows(3); P.OpenView(); P.GoToKey('R2'); P.DelCur.Invoke();
    end;
    procedure SecondAction()
    var P: TestPage "FL Temp RunObj List";
    begin
        Rows(3); P.OpenView(); P.GoToKey('R2'); P.DelCur.Invoke();
    end;
    procedure CalleeWrites()
    var P: TestPage "FL Temp Touch List";
    begin
        Rows(3); P.OpenView(); P.GoToKey('R2'); P.DelCur.Invoke();
    end;
    procedure StoredList(): Text
    var P: TestPage "FL Stored List";
    begin
        Rows(3); P.OpenView(); P.GoToKey('R2'); P.DelCur.Invoke();
        exit(P."Code".Value());
    end;
}
"#;

/// ReopenSameVariableState.al:504-516: `del-1` `[del(R2) agcr(R1) agcr(R1)]`, `cursor-1` R1, then Next lands on R3; the same on
/// the second open.
#[test]
fn the_list_lands_on_the_row_before_the_deleted_one() {
    take_hits();
    assert_eq!(
        text(run_in(SRC, "T", "Probe")),
        "[open agcr(R3) agcr(R1) agcr(R2)] R1 [del(R2) agcr(R1) agcr(R1)] R3 [agcr(R3)] [close] R3 [open agcr(R3) agcr(R1) agcr(R2)] R1 [del(R2) agcr(R1) agcr(R1)] R3 [agcr(R3)] [close]"
    );
    assert!(take_hits().contains(&Assumption::TempReadOnlyListGoneMiddleRowLandsBefore));
}

/// The neighbors of the probe's cell on a list over a temporary table: refused.
#[test]
fn unmeasured_neighbors_are_refused() {
    // The review's neighbors (al-runner PR 7946, round 1): `CurrPage.Update(false)` after the delete (T1), a change of another
    // row in the action (T2), OnOpenPage ending on the first row (T3), a call in the argument of the action's log call.
    // Round 2: a second action without a trigger (TL1), a log procedure that also writes a stored row (L1).
    for proc in [
        "First",
        "Last",
        "FourRows",
        "OpenEdit",
        "EditableList",
        "NextBefore",
        "UpdateInAction",
        "ModifyOtherInAction",
        "OpenOnFirst",
        "ArgCallInAction",
        "SecondAction",
        "CalleeWrites",
    ] {
        let r = run_in(SRC, "T", proc);
        assert!(
            matches!(&r, Err(Signal::Unsupported(why)) if why.contains("a list over a temporary table after its action deleted")),
            "{proc}: {r:?}"
        );
    }
}

/// A list over a stored table keeps main's landing on the next row (pages/W1Update.al:577).
#[test]
fn a_stored_list_keeps_mains_next_row() {
    take_hits();
    assert_eq!(text(run_in(SRC, "T", "StoredList")), "R3");
    assert!(!take_hits().contains(&Assumption::TempReadOnlyListGoneMiddleRowLandsBefore));
}
