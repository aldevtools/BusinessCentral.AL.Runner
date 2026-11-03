//! TestField.Lookup through OnLookup, whose OnValidate calls `CurrPage.Update(false)`, on a List host with
//! OnAfterGetCurrRecord whose ListPart links three fields, when the host has three or more parts: two or three unlinked
//! ListParts before, between or after the linked one, with or without a FactBox CardPart (al-runner issue 6806, the
//! reporter's repro; real BC shows `R2` on every page).

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50100 "Repro Lookup Setting" { fields { field(1; "Primary Key"; Code[10]) { } field(2; "Resource No."; Code[20]) { } } keys { key(PK; "Primary Key") { } } }
table 50101 "Repro Lookup Resource"
{
    LookupPageId = "Repro Lookup Resource List";
    fields { field(1; "No."; Code[20]) { } field(2; "Group Code"; Code[20]) { } field(3; "Region Code"; Code[20]) { } }
    keys { key(PK; "No.") { } }
}
page 50102 "Repro Lookup Resource List"
{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Lines) { field("No."; Rec."No.") { } field("Group Code"; Rec."Group Code") { } } } }
}
page 50103 "Repro Lookup Resource Part"
{
    PageType = ListPart; SourceTable = "Repro Lookup Resource"; Editable = false;
    layout { area(Content) { repeater(Lines) { field("No."; Rec."No.") { } field("Group Code"; Rec."Group Code") { } } } }
}
page 50111 "Repro Lookup Card Part"
{
    PageType = CardPart; SourceTable = "Repro Lookup Resource";
    layout { area(Content) { field("No."; Rec."No.") { } } }
}
page 50120 "Repro AB L Schedule"
{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None;
    layout {
        area(Content) {
        group(Filters) { field(ResourceNo; Setting."Resource No.") {
            trigger OnLookup(var Text: Text): Boolean
            var Resource: Record "Repro Lookup Resource";
            begin
                if Page.RunModal(Page::"Repro Lookup Resource List", Resource) <> Action::LookupOK then exit(false);
                Text := Resource."No."; exit(true);
            end;
            trigger OnValidate() begin Setting.Modify(); CurrPage.Update(false); end;
        } }
        repeater(Lines) { field("No."; Rec."No.") { } field(CurrentNo; CurrentNo) { } }
        part(OtherA; "Repro Lookup Resource Part") {  }
        part(OtherB; "Repro Lookup Resource Part") {  }
        part(Resources; "Repro Lookup Resource Part") { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code"); }
        }
    }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin CurrentNo := Rec."No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting";
}
page 50121 "Repro L AB Schedule"
{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None;
    layout {
        area(Content) {
        group(Filters) { field(ResourceNo; Setting."Resource No.") {
            trigger OnLookup(var Text: Text): Boolean
            var Resource: Record "Repro Lookup Resource";
            begin
                if Page.RunModal(Page::"Repro Lookup Resource List", Resource) <> Action::LookupOK then exit(false);
                Text := Resource."No."; exit(true);
            end;
            trigger OnValidate() begin Setting.Modify(); CurrPage.Update(false); end;
        } }
        repeater(Lines) { field("No."; Rec."No.") { } field(CurrentNo; CurrentNo) { } }
        part(Resources; "Repro Lookup Resource Part") { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code"); }
        part(OtherA; "Repro Lookup Resource Part") {  }
        part(OtherB; "Repro Lookup Resource Part") {  }
        }
    }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin CurrentNo := Rec."No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting";
}
page 50122 "Repro A L B Schedule"
{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None;
    layout {
        area(Content) {
        group(Filters) { field(ResourceNo; Setting."Resource No.") {
            trigger OnLookup(var Text: Text): Boolean
            var Resource: Record "Repro Lookup Resource";
            begin
                if Page.RunModal(Page::"Repro Lookup Resource List", Resource) <> Action::LookupOK then exit(false);
                Text := Resource."No."; exit(true);
            end;
            trigger OnValidate() begin Setting.Modify(); CurrPage.Update(false); end;
        } }
        repeater(Lines) { field("No."; Rec."No.") { } field(CurrentNo; CurrentNo) { } }
        part(OtherA; "Repro Lookup Resource Part") {  }
        part(Resources; "Repro Lookup Resource Part") { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code"); }
        part(OtherB; "Repro Lookup Resource Part") {  }
        }
    }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin CurrentNo := Rec."No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting";
}
page 50123 "Repro AB L Card Schedule"
{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None;
    layout {
        area(Content) {
        group(Filters) { field(ResourceNo; Setting."Resource No.") {
            trigger OnLookup(var Text: Text): Boolean
            var Resource: Record "Repro Lookup Resource";
            begin
                if Page.RunModal(Page::"Repro Lookup Resource List", Resource) <> Action::LookupOK then exit(false);
                Text := Resource."No."; exit(true);
            end;
            trigger OnValidate() begin Setting.Modify(); CurrPage.Update(false); end;
        } }
        repeater(Lines) { field("No."; Rec."No.") { } field(CurrentNo; CurrentNo) { } }
        part(OtherA; "Repro Lookup Resource Part") {  }
        part(OtherB; "Repro Lookup Resource Part") {  }
        part(Resources; "Repro Lookup Resource Part") { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code"); }
        }
        area(FactBoxes) {
        part(Details; "Repro Lookup Card Part") { SubPageLink = "No." = field("No."); }
        }
    }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin CurrentNo := Rec."No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting";
}
page 50124 "Repro ABC L Schedule"
{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None;
    layout {
        area(Content) {
        group(Filters) { field(ResourceNo; Setting."Resource No.") {
            trigger OnLookup(var Text: Text): Boolean
            var Resource: Record "Repro Lookup Resource";
            begin
                if Page.RunModal(Page::"Repro Lookup Resource List", Resource) <> Action::LookupOK then exit(false);
                Text := Resource."No."; exit(true);
            end;
            trigger OnValidate() begin Setting.Modify(); CurrPage.Update(false); end;
        } }
        repeater(Lines) { field("No."; Rec."No.") { } field(CurrentNo; CurrentNo) { } }
        part(OtherA; "Repro Lookup Resource Part") {  }
        part(OtherB; "Repro Lookup Resource Part") {  }
        part(OtherC; "Repro Lookup Resource Part") {  }
        part(Resources; "Repro Lookup Resource Part") { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code"); }
        }
    }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin CurrentNo := Rec."No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting";
}
codeunit 50110 "Repro Lookup Link Tests"
{
    Subtype = Test;
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure PartsABL()
    var Schedule: TestPage "Repro AB L Schedule";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup();
        AssertPicked(Schedule.ResourceNo.Value());
        AssertPart(Schedule.OtherA."No.".Value()); AssertPart(Schedule.OtherB."No.".Value()); AssertPart(Schedule.Resources."No.".Value());
        Schedule.Close();
    end;
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure PartsLAB()
    var Schedule: TestPage "Repro L AB Schedule";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup();
        AssertPicked(Schedule.ResourceNo.Value());
        AssertPart(Schedule.Resources."No.".Value()); AssertPart(Schedule.OtherA."No.".Value()); AssertPart(Schedule.OtherB."No.".Value());
        Schedule.Close();
    end;
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure PartsALB()
    var Schedule: TestPage "Repro A L B Schedule";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup();
        AssertPicked(Schedule.ResourceNo.Value());
        AssertPart(Schedule.OtherA."No.".Value()); AssertPart(Schedule.Resources."No.".Value()); AssertPart(Schedule.OtherB."No.".Value());
        Schedule.Close();
    end;
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure PartsABLK()
    var Schedule: TestPage "Repro AB L Card Schedule";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup();
        AssertPicked(Schedule.ResourceNo.Value());
        AssertPart(Schedule.OtherA."No.".Value()); AssertPart(Schedule.OtherB."No.".Value()); AssertPart(Schedule.Resources."No.".Value()); AssertPart(Schedule.Details."No.".Value());
        Schedule.Close();
    end;
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure PartsABCL()
    var Schedule: TestPage "Repro ABC L Schedule";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup();
        AssertPicked(Schedule.ResourceNo.Value());
        AssertPart(Schedule.OtherA."No.".Value()); AssertPart(Schedule.OtherB."No.".Value()); AssertPart(Schedule.OtherC."No.".Value()); AssertPart(Schedule.Resources."No.".Value());
        Schedule.Close();
    end;
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure PartNextAfterTheLookup()
    var Schedule: TestPage "Repro AB L Schedule";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup(); Schedule.OtherB.Next();
    end;
    [ModalPageHandler]
    procedure PickSecondResource(var ResourceList: TestPage "Repro Lookup Resource List")
    begin ResourceList.GoToKey('R2'); ResourceList.OK().Invoke(); end;
    local procedure Setup()
    var Setting: Record "Repro Lookup Setting"; Resource: Record "Repro Lookup Resource";
    begin
        Setting.DeleteAll(); Resource.DeleteAll();
        InsertResource(Resource, 'R1', 'G1'); InsertResource(Resource, 'R2', 'G1'); InsertResource(Resource, 'R3', 'G2');
    end;
    local procedure InsertResource(var Resource: Record "Repro Lookup Resource"; ResourceNo: Code[20]; GroupCode: Code[20])
    begin Resource.Init(); Resource."No." := ResourceNo; Resource."Group Code" := GroupCode; Resource.Insert(); end;
    local procedure AssertPicked(Actual: Text)
    begin if Actual <> 'R2' then Error('Resource No. shows [%1] after the lookup.', Actual); end;
    local procedure AssertPart(Actual: Text)
    begin if Actual <> 'R1' then Error('A part shows [%1], not its row R1.', Actual); end;
}
"#;

#[test]
fn issue_6806_a_lookup_on_a_host_with_three_or_more_parts_shows_the_picked_value() {
    let o = outcomes_in(SRC, "Repro Lookup Link Tests");
    for n in ["PartsABL", "PartsLAB", "PartsALB", "PartsABLK", "PartsABCL"] {
        let g = o.iter().find(|(name, _)| name == n).map(|(_, x)| format!("{x:?}"));
        assert_eq!(g.as_deref(), Some("Pass"), "{n}");
    }
}

/// R6806LookupParts.al R6806_ABL_OtherBNextFirst: the reads that follow a move of a part are not built for three parts.
#[test]
fn issue_6806_a_part_move_after_the_lookup_on_three_parts_is_refused() {
    let o = outcomes_in(SRC, "Repro Lookup Link Tests");
    let g = o
        .iter()
        .find(|(n, _)| n == "PartNextAfterTheLookup")
        .map(|(_, x)| format!("{x:?}"))
        .unwrap_or_default();
    assert!(
        g.contains("NotRun") && g.contains("three or more parts after TestField.Lookup"),
        "{g}"
    );
}
