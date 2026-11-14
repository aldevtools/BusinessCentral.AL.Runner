//! The calls after an action left one row behind the new-row line of an empty List (`empty_list_one_row.rs`, al-runner issue
//! 7478). The pinned cells run; each neighbor flips one fact of pages/actions/ActionReportInsertsOneRowOnEmptyList.al and keeps
//! main's refusal.

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50480 "OR Line"
{
    fields { field(1; "Parent No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Description; Text[30]) { } }
    keys { key(PK; "Parent No.", "Line No.") { } }
}
report 50490 "OR Report"
{
    ProcessingOnly = true; UsageCategory = None;
    trigger OnPreReport()
        var L: Record "OR Line";
        begin
            L.Init(); L."Parent No." := 'P1'; L."Line No." := LineNo; L.Description := 'by report';
            if WithTrigger then L.Insert(true) else L.Insert();
            if AlsoModify then begin L.Description := 'modified'; L.Modify(); end;
            if AlsoCommit then Commit();
        end;
    requestpage { layout { area(Content) { field(Ctl; Dummy) { ApplicationArea = All; } } } }
    var LineNo: Integer; WithTrigger: Boolean; AlsoModify: Boolean; AlsoCommit: Boolean; Dummy: Integer;
    procedure Setup(NewLineNo: Integer; Trig: Boolean; Modify: Boolean; CommitToo: Boolean)
    begin LineNo := NewLineNo; WithTrigger := Trig; AlsoModify := Modify; AlsoCommit := CommitToo; end;
}
codeunit 50491 "OR Nothing" { trigger OnRun() begin end; }
codeunit 50492 "OR Mgt"
{
    // 1 the pinned shape (a report with a request page, one plain Insert of key P1/10000); 2 Insert(true); 3 Insert then Modify;
    // 4 Commit after the Insert; 5 key below 0; 6 a report without request page; 7 then Codeunit.Run; 8 a direct Insert.
    procedure AddLines(Mode: Integer)
    var R: Report "OR Report"; L: Record "OR Line";
    begin
        case Mode of
            1: begin R.Setup(10000, false, false, false); R.Run(); end;
            2: begin R.Setup(10000, true, false, false); R.Run(); end;
            3: begin R.Setup(10000, false, true, false); R.Run(); end;
            4: begin R.Setup(10000, false, false, true); R.Run(); end;
            5: begin R.Setup(-5, false, false, false); R.Run(); end;
            6: begin R.Setup(10000, false, false, false); R.UseRequestPage(false); R.RunModal(); end;
            7: begin R.Setup(10000, false, false, false); R.Run(); Codeunit.Run(Codeunit::"OR Nothing"); end;
            8: begin L."Parent No." := 'P1'; L."Line No." := 10000; L.Insert(); end;
        end;
    end;
}
page 50481 "OR Delayed"
{
    PageType = List; SourceTable = "OR Line"; DelayedInsert = true; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Parent No."; Rec."Parent No.") { ApplicationArea = All; } field("Line No."; Rec."Line No.") { ApplicationArea = All; } field(Description; Rec.Description) { ApplicationArea = All; } } } }
    actions
    {
        area(Processing)
        {
            action(Add1) { ApplicationArea = All; trigger OnAction() begin Mgt.AddLines(1); end; }
            action(Add2) { ApplicationArea = All; trigger OnAction() begin Mgt.AddLines(2); end; }
            action(Add3) { ApplicationArea = All; trigger OnAction() begin Mgt.AddLines(3); end; }
            action(Add4) { ApplicationArea = All; trigger OnAction() begin Mgt.AddLines(4); end; }
            action(Add5) { ApplicationArea = All; trigger OnAction() begin Mgt.AddLines(5); end; }
            action(Add6) { ApplicationArea = All; trigger OnAction() begin Mgt.AddLines(6); end; }
            action(Add7) { ApplicationArea = All; trigger OnAction() begin Mgt.AddLines(7); end; }
            action(Add8) { ApplicationArea = All; trigger OnAction() begin Mgt.AddLines(8); end; }
            action(AddUpdate) { ApplicationArea = All; trigger OnAction() begin Mgt.AddLines(1); CurrPage.Update(false); end; }
            action(AddSetRec) { ApplicationArea = All; trigger OnAction() begin Mgt.AddLines(1); Rec.Description := 'rec written'; end; }
        }
    }
    var Mgt: Codeunit "OR Mgt";
}
page 50482 "OR Plain"
{
    PageType = List; SourceTable = "OR Line"; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Parent No."; Rec."Parent No.") { ApplicationArea = All; } field("Line No."; Rec."Line No.") { ApplicationArea = All; } } } }
    actions { area(Processing) { action(Add1) { ApplicationArea = All; trigger OnAction() begin Mgt.AddLines(1); end; } } }
    var Mgt: Codeunit "OR Mgt";
}
page 50483 "OR Trigger"
{
    PageType = List; SourceTable = "OR Line"; DelayedInsert = true; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { ApplicationArea = All; } } } }
    actions { area(Processing) { action(Add1) { ApplicationArea = All; trigger OnAction() begin Mgt.AddLines(1); end; } } }
    var Mgt: Codeunit "OR Mgt";
    trigger OnAfterGetRecord() begin end;
}
codeunit 50485 T
{
    Subtype = Test;
    [RequestPageHandler]
    procedure H(var RP: TestRequestPage "OR Report")
    begin
        RP.OK().Invoke();
    end;
    local procedure Stored(): Integer
    var Line: Record "OR Line";
    begin
        exit(Line.Count());
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_Next_Ok()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add1.Invoke();
        if Pg.Next() then Error('Next moved');
        if Pg."Line No.".Value <> '0' then Error('shown %1', Pg."Line No.".Value);
        Pg.OK().Invoke();
        if Stored() <> 1 then Error('stored %1', Stored());
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_Previous_Ok()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add1.Invoke();
        if not Pg.Previous() then Error('Previous did not move');
        if Pg."Line No.".Value <> '10000' then Error('shown %1', Pg."Line No.".Value);
        Pg.OK().Invoke();
        if Stored() <> 1 then Error('stored %1', Stored());
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Plain_Ok()
    var Pg: TestPage "OR Plain"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add1.Invoke();
        Pg.OK().Invoke();
        if Stored() <> 1 then Error('stored %1', Stored());
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure PageWithTrigger_Ok_Refused()
    var Pg: TestPage "OR Trigger"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add1.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Plain_Next_Refused()
    var Pg: TestPage "OR Plain"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add1.Invoke();
        Pg.Next();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_RecWritten_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.AddSetRec.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_NoFilter_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Add1.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_OtherFilter_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter(Description, '');
        Pg.Add1.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_NewBeforeAction_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.New();
        Pg.Add1.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_Action_InsertTrue_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add2.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_Action_InsertThenModify_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add3.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_Action_Commit_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add4.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_Action_KeyBelowZero_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add5.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_Action_CodeunitRun_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add7.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    procedure Delayed_Action_DirectInsert_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add8.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    procedure Delayed_Action_NoRequestPage_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add6.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_Action_CurrPageUpdate_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.AddUpdate.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_Next_SetValue_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add1.Invoke();
        Pg.Next();
        Pg.Description.SetValue('typed');
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_Previous_Next_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add1.Invoke();
        Pg.Previous();
        Pg.Next();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_Last_Close_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add1.Invoke();
        Pg.Last();
        Pg.Close();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Delayed_GoToKeyOtherKey_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll(); Commit();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.Add1.Invoke();
        Pg.GoToKey('P1', 20000);
    end;
}
"#;

#[test]
fn one_row_behind_empty_list() {
    let outcomes = outcomes_in(SRC, "T");
    let get = |name: &str| {
        let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
        format!("{o:?}")
    };
    let mut wrong = Vec::new();
    for pass in ["Delayed_Next_Ok", "Delayed_Previous_Ok", "Plain_Ok"] {
        if get(pass) != "Some(Pass)" {
            wrong.push(format!("{pass}: {}", get(pass)));
        }
    }
    // Each neighbor keeps main's refusal of a call after the action.
    for refused in [
        "PageWithTrigger_Ok_Refused",
        "Plain_Next_Refused",
        "Delayed_RecWritten_Ok_Refused",
        "Delayed_NoFilter_Ok_Refused",
        "Delayed_OtherFilter_Ok_Refused",
        "Delayed_NewBeforeAction_Ok_Refused",
        "Delayed_GoToKeyOtherKey_Refused",
        // What the action did (the report's facts of ActionReportInsertsOneRowOnEmptyList.al: one plain Insert, nothing else).
        "Delayed_Action_InsertTrue_Ok_Refused",
        "Delayed_Action_InsertThenModify_Ok_Refused",
        "Delayed_Action_Commit_Ok_Refused",
        "Delayed_Action_KeyBelowZero_Ok_Refused",
        "Delayed_Action_CodeunitRun_Ok_Refused",
        "Delayed_Action_DirectInsert_Ok_Refused",
        "Delayed_Action_NoRequestPage_Ok_Refused",
        "Delayed_Action_CurrPageUpdate_Ok_Refused",
    ] {
        let o = get(refused);
        if !(o.contains("NotRun") && o.contains("after an action stored a row behind the new-row line")) {
            wrong.push(format!("{refused}: {o}"));
        }
    }
    // The pinned call ran; what follows it is not one of the pin's next calls.
    for refused in [
        "Delayed_Next_SetValue_Refused",
        "Delayed_Previous_Next_Refused",
        "Delayed_Last_Close_Refused",
    ] {
        let o = get(refused);
        if !(o.contains("NotRun") && o.contains("after the call that followed an action that stored one row")) {
            wrong.push(format!("{refused}: {o}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
