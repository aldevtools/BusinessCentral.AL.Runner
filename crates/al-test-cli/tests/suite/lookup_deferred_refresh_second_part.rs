//! TestField.Lookup through OnLookup, whose OnValidate calls `CurrPage.Update(false)`, on a List host with
//! OnAfterGetCurrRecord whose ListPart links three fields, when the host has a second part: another ListPart with no link,
//! or a CardPart in the FactBox area (al-runner issue 6639, the reporter's repro; real BC shows `R2`).

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
page 50107 "Repro Two Part Schedule"
{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None;
    layout { area(Content) {
        group(Filters) { field(ResourceNo; Setting."Resource No.") {
            Caption = 'Resource No.';
            trigger OnLookup(var Text: Text): Boolean
            var Resource: Record "Repro Lookup Resource";
            begin
                if Page.RunModal(Page::"Repro Lookup Resource List", Resource) <> Action::LookupOK then exit(false);
                Text := Resource."No."; exit(true);
            end;
            trigger OnValidate() begin Setting.Modify(); CurrPage.Update(false); end;
        } }
        repeater(Lines) { field("No."; Rec."No.") { } field(CurrentNo; CurrentNo) { } }
        part(OtherResources; "Repro Lookup Resource Part") { }
        part(Resources; "Repro Lookup Resource Part")
        {
            SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code");
        }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin CurrentNo := Rec."No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting";
}
page 50108 "Repro Card Part Schedule"
{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None;
    layout {
        area(Content) {
            group(Filters) { field(ResourceNo; Setting."Resource No.") {
                Caption = 'Resource No.';
                trigger OnLookup(var Text: Text): Boolean
                var Resource: Record "Repro Lookup Resource";
                begin
                    if Page.RunModal(Page::"Repro Lookup Resource List", Resource) <> Action::LookupOK then exit(false);
                    Text := Resource."No."; exit(true);
                end;
                trigger OnValidate() begin Setting.Modify(); CurrPage.Update(false); end;
            } }
            repeater(Lines) { field("No."; Rec."No.") { } field(CurrentNo; CurrentNo) { } }
            part(Resources; "Repro Lookup Resource Part")
            {
                SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code");
            }
        }
        area(FactBoxes) { part(Details; "Repro Lookup Card Part") { SubPageLink = "No." = field("No."); } }
    }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin CurrentNo := Rec."No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting";
}
page 50112 "Repro Part Then Other Schedule"
{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None;
    layout { area(Content) {
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
        part(Resources; "Repro Lookup Resource Part")
        {
            SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code");
        }
        part(OtherResources; "Repro Lookup Resource Part") { }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin CurrentNo := Rec."No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting";
}
codeunit 50110 "Repro Lookup Link Tests"
{
    Subtype = Test;
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure SecondListPartShowsThePickedResource()
    var Schedule: TestPage "Repro Two Part Schedule";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup();
        AssertPickedResource(Schedule.ResourceNo.Value()); Schedule.Close();
    end;
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure CardFactBoxShowsThePickedResource()
    var Schedule: TestPage "Repro Card Part Schedule";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup();
        AssertPickedResource(Schedule.ResourceNo.Value()); Schedule.Close();
    end;
    // Y63_PartThenOther_LinkedPartNextFirst `after lookup log`: with the linked part first, reading its controls ran the refresh.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure LinkedPartFirst()
    var Schedule: TestPage "Repro Part Then Other Schedule";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup();
        AssertPickedResource(Schedule.ResourceNo.Value());
    end;
    // Y63_TwoPart_LinkedPartNextFirst `part next log`: the first part call runs the refresh with every part, not built.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure PartNextAfterTheLookup()
    var Schedule: TestPage "Repro Two Part Schedule";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup(); Schedule.Resources.Next();
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
    local procedure AssertPickedResource(ActualResourceNo: Text)
    begin if ActualResourceNo <> 'R2' then Error('Resource No. shows [%1] after the lookup.', ActualResourceNo); end;
}
"#;

#[test]
fn issue_6639_a_lookup_on_a_host_with_a_second_part_shows_the_picked_value() {
    let o = outcomes_in(SRC, "Repro Lookup Link Tests");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    assert_eq!(get("SecondListPartShowsThePickedResource"), "Some(Pass)");
    assert_eq!(get("CardFactBoxShowsThePickedResource"), "Some(Pass)");
    let g = get("LinkedPartFirst");
    assert!(
        g.contains("NotRun") && g.contains("whose refresh of the page is not pinned"),
        "{g}"
    );
    let g = get("PartNextAfterTheLookup");
    assert!(
        g.contains("NotRun") && g.contains("BC runs the refresh of the OnValidate's CurrPage.Update(false) first"),
        "{g}"
    );
}

/// The reporter's two tests pass flagged: Y63's values are not on the pinned corpus yet.
#[test]
fn issue_6639_the_second_part_lookups_mark_their_assumption() {
    let l = al_testkit::compile(SRC);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "Repro Lookup Link Tests")
        .unwrap();
    let mut hits = Vec::new();
    al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        al_testrun::isolation::TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    for n in [
        "SecondListPartShowsThePickedResource",
        "CardFactBoxShowsThePickedResource",
    ] {
        let h = hits
            .iter()
            .find(|(t, _)| t == n)
            .map(|(_, h)| format!("{h:?}"))
            .unwrap_or_default();
        assert!(h.contains("LookupLinkDefersRefreshWithSecondPart"), "{n} marks {h}");
    }
}
