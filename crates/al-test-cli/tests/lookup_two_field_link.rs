//! TestField.Lookup through OnLookup on a page whose ListPart has a SubPageLink of two `field(..)` entries shows the picked
//! value (pages/lt-wb-6290/WbLookupListPartTests.al:21-31 OnePartWithTwoLinkedFieldsShowsThePickedResource; one entry:
//! pages/lt-sc-outside-pages/ScLookupGlobalRecordField.al; al-runner issue 6290). Three entries are not pinned and are refused.

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50100 "LR Setting" { fields { field(1; "Primary Key"; Code[10]) { } field(2; "Resource No."; Code[20]) { } } keys { key(PK; "Primary Key") { } } }
table 50101 "LR Resource" { LookupPageId = "LR Resource List"; fields { field(1; "No."; Code[20]) { } field(2; "Group Code"; Code[20]) { } field(3; Name; Code[20]) { } } keys { key(PK; "No.") { } } }
page 50102 "LR Resource List" { PageType = List; SourceTable = "LR Resource"; UsageCategory = None; Editable = false; layout { area(Content) { repeater(Lines) { field("No."; Rec."No.") { } } } } }
page 50103 "LR Resource Part"
{
    PageType = ListPart; SourceTable = "LR Resource"; Editable = false;
    layout { area(Content) { field(Scope; Scope) { } repeater(Lines) { } } }
    var Scope: Code[20];
}
page 50110 "Sched One"
{
    PageType = List; SourceTable = "LR Resource"; UsageCategory = None;
    layout { area(Content) {
        group(Filters) { field(ResourceNo; Setting."Resource No.") {
            trigger OnLookup(var Text: Text): Boolean
            var Resource: Record "LR Resource";
            begin
                if Page.RunModal(Page::"LR Resource List", Resource) <> Action::LookupOK then exit(false);
                Text := Resource."No."; exit(true);
            end;
            trigger OnValidate() begin Setting.Modify(); CurrPage.Update(false); end;
        } }
        repeater(Lines) { }
        part(Resources; "LR Resource Part") { SubPageLink = "No." = field("No."); }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    var Setting: Record "LR Setting";
}
page 50111 "Sched Two"
{
    PageType = List; SourceTable = "LR Resource"; UsageCategory = None;
    layout { area(Content) {
        group(Filters) { field(ResourceNo; Setting."Resource No.") {
            trigger OnLookup(var Text: Text): Boolean
            var Resource: Record "LR Resource";
            begin
                if Page.RunModal(Page::"LR Resource List", Resource) <> Action::LookupOK then exit(false);
                Text := Resource."No."; exit(true);
            end;
            trigger OnValidate() begin Setting.Modify(); CurrPage.Update(false); end;
        } }
        repeater(Lines) { }
        part(Resources; "LR Resource Part") { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"); }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    var Setting: Record "LR Setting";
}
page 50112 "Sched Three"
{
    PageType = List; SourceTable = "LR Resource"; UsageCategory = None;
    layout { area(Content) {
        group(Filters) { field(ResourceNo; Setting."Resource No.") {
            trigger OnLookup(var Text: Text): Boolean
            var Resource: Record "LR Resource";
            begin
                if Page.RunModal(Page::"LR Resource List", Resource) <> Action::LookupOK then exit(false);
                Text := Resource."No."; exit(true);
            end;
            trigger OnValidate() begin Setting.Modify(); CurrPage.Update(false); end;
        } }
        repeater(Lines) { }
        part(Resources; "LR Resource Part") { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Name" = field("Name"); }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    var Setting: Record "LR Setting";
}
codeunit 50120 T
{
    Subtype = Test;
    [Test] [HandlerFunctions('PickSecond')]
    procedure LinkOne()
    var S: TestPage "Sched One";
    begin
        Setup(); S.OpenEdit(); S.ResourceNo.Lookup();
        if S.ResourceNo.Value() <> 'R2' then Error('shows [%1]', S.ResourceNo.Value());
        S.Close();
    end;
    [Test] [HandlerFunctions('PickSecond')]
    procedure LinkTwo()
    var S: TestPage "Sched Two";
    begin
        Setup(); S.OpenEdit(); S.ResourceNo.Lookup();
        if S.ResourceNo.Value() <> 'R2' then Error('shows [%1]', S.ResourceNo.Value());
        S.Close();
    end;
    [Test] [HandlerFunctions('PickSecond')]
    procedure LinkThree()
    var S: TestPage "Sched Three";
    begin
        Setup(); S.OpenEdit(); S.ResourceNo.Lookup();
        if S.ResourceNo.Value() <> 'R2' then Error('shows [%1]', S.ResourceNo.Value());
        S.Close();
    end;
    local procedure Setup()
    var Setting: Record "LR Setting"; R: Record "LR Resource";
    begin
        Setting.DeleteAll(); R.DeleteAll();
        R."No." := 'R1'; R."Group Code" := 'G1'; R.Insert();
        R."No." := 'R2'; R."Group Code" := 'G1'; R.Insert();
        R."No." := 'R3'; R."Group Code" := 'G2'; R.Insert();
    end;
    [ModalPageHandler] procedure PickSecond(var L: TestPage "LR Resource List") begin L.GoToKey('R2'); L.OK().Invoke(); end;
}
"#;

#[test]
fn a_lookup_on_a_page_whose_part_links_two_fields_shows_the_picked_value() {
    let o = outcomes_in(SRC, "T");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    assert_eq!(get("LinkOne"), "Some(Pass)");
    assert_eq!(get("LinkTwo"), "Some(Pass)");
    let g = get("LinkThree");
    assert!(
        g.contains("NotRun") && g.contains("whose refresh of the page is not pinned"),
        "{g}"
    );
}
