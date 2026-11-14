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
page 50481 "OR Delayed"
{
    PageType = List; SourceTable = "OR Line"; DelayedInsert = true; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Parent No."; Rec."Parent No.") { ApplicationArea = All; } field("Line No."; Rec."Line No.") { ApplicationArea = All; } field(Description; Rec.Description) { ApplicationArea = All; } } } }
    actions
    {
        area(Processing)
        {
            action(AddOne) { ApplicationArea = All; trigger OnAction() var L: Record "OR Line"; begin L."Parent No." := 'P1'; L."Line No." := 10000; L.Insert(); end; }
            action(AddOneSetRec) { ApplicationArea = All; trigger OnAction() var L: Record "OR Line"; begin L."Parent No." := 'P1'; L."Line No." := 10000; L.Insert(); Rec.Description := 'rec written'; end; }
        }
    }
}
page 50482 "OR Plain"
{
    PageType = List; SourceTable = "OR Line"; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Parent No."; Rec."Parent No.") { ApplicationArea = All; } field("Line No."; Rec."Line No.") { ApplicationArea = All; } } } }
    actions { area(Processing) { action(AddOne) { ApplicationArea = All; trigger OnAction() var L: Record "OR Line"; begin L."Parent No." := 'P1'; L."Line No." := 10000; L.Insert(); end; } } }
}
page 50483 "OR Trigger"
{
    PageType = List; SourceTable = "OR Line"; DelayedInsert = true; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { ApplicationArea = All; } } } }
    actions { area(Processing) { action(AddOne) { ApplicationArea = All; trigger OnAction() var L: Record "OR Line"; begin L."Parent No." := 'P1'; L."Line No." := 10000; L.Insert(); end; } } }
    trigger OnAfterGetRecord() begin end;
}
codeunit 50485 T
{
    Subtype = Test;
    local procedure Stored(): Integer
    var Line: Record "OR Line";
    begin
        exit(Line.Count());
    end;
    [Test]
    procedure Delayed_Next_Ok()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.AddOne.Invoke();
        if Pg.Next() then Error('Next moved');
        if Pg."Line No.".Value <> '0' then Error('shown %1', Pg."Line No.".Value);
        Pg.OK().Invoke();
        if Stored() <> 1 then Error('stored %1', Stored());
    end;
    [Test]
    procedure Delayed_Previous_Ok()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.AddOne.Invoke();
        if not Pg.Previous() then Error('Previous did not move');
        if Pg."Line No.".Value <> '10000' then Error('shown %1', Pg."Line No.".Value);
        Pg.OK().Invoke();
        if Stored() <> 1 then Error('stored %1', Stored());
    end;
    [Test]
    procedure Plain_Ok()
    var Pg: TestPage "OR Plain"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.AddOne.Invoke();
        Pg.OK().Invoke();
        if Stored() <> 1 then Error('stored %1', Stored());
    end;
    [Test]
    procedure PageWithTrigger_Ok_Refused()
    var Pg: TestPage "OR Trigger"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.AddOne.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    procedure Plain_Next_Refused()
    var Pg: TestPage "OR Plain"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.AddOne.Invoke();
        Pg.Next();
    end;
    [Test]
    procedure Delayed_RecWritten_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.AddOneSetRec.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    procedure Delayed_NoFilter_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.AddOne.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    procedure Delayed_OtherFilter_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.Filter.SetFilter(Description, '');
        Pg.AddOne.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    procedure Delayed_NewBeforeAction_Ok_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.New();
        Pg.AddOne.Invoke();
        Pg.OK().Invoke();
    end;
    [Test]
    procedure Delayed_Next_SetValue_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.AddOne.Invoke();
        Pg.Next();
        Pg.Description.SetValue('typed');
    end;
    [Test]
    procedure Delayed_Previous_Next_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.AddOne.Invoke();
        Pg.Previous();
        Pg.Next();
    end;
    [Test]
    procedure Delayed_Last_Close_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.AddOne.Invoke();
        Pg.Last();
        Pg.Close();
    end;
    [Test]
    procedure Delayed_GoToKeyOtherKey_Refused()
    var Pg: TestPage "OR Delayed"; Line: Record "OR Line";
    begin
        Line.DeleteAll();
        Pg.OpenEdit();
        Pg.Filter.SetFilter("Parent No.", 'P1');
        Pg.AddOne.Invoke();
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
