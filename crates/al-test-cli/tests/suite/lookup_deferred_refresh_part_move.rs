//! The refresh a TestField.Lookup left for later (al-runner issue 6519) on a List host with OnAfterGetCurrRecord only, whose
//! ListPart (no row triggers) links three fields: the first TestPart Next, First, Previous or Last runs it, a second Lookup runs
//! it first (host only), and a cancelled Lookup leaves nothing pending (corpus writer Y54, corpus PR 990,
//! pages/lt-y15/Y15LookupThreeFieldLink.al:991 Y54_Y15Host_PartWalkAfterLookup; :213, :272, :297 the Y15 cases).

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
page 50104 "Repro Three Link Schedule"
{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None;
    layout { area(Content) {
        group(Filters) {
            field(ResourceNo; Setting."Resource No.")
            {
                trigger OnLookup(var Text: Text): Boolean
                var Resource: Record "Repro Lookup Resource";
                begin
                    Log.Add('lookup');
                    if Page.RunModal(Page::"Repro Lookup Resource List", Resource) <> Action::LookupOK then exit(false);
                    Text := Resource."No."; exit(true);
                end;
                trigger OnValidate() begin Log.Add('validate ' + Setting."Resource No."); Setting.Modify(); CurrPage.Update(false); end;
            }
        }
        repeater(Lines) { field("No."; Rec."No.") { } }
        part(Resources; "Repro Lookup Resource Part")
        { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code"); }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr ' + Rec."No."); end;
    var Setting: Record "Repro Lookup Setting"; Log: Codeunit "Repro Log";
}
codeunit 50105 "Repro Log"
{
    SingleInstance = true;
    procedure Add(Entry: Text) begin if Entries <> '' then Entries += ';'; Entries += Entry; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
    var Entries: Text;
}
codeunit 50110 "Repro Deferred Refresh Tests"
{
    Subtype = Test;
    var Log: Codeunit "Repro Log"; PickCount: Integer;
    // :991 Y54_Y15Host_PartWalkAfterLookup.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure PartWalkAfterLookup()
    var S: TestPage "Repro Three Link Schedule";
    begin
        Setup(); S.OpenEdit(); Log.Take();
        S.ResourceNo.Lookup();
        Check('lookup log', Log.Take(), 'lookup;validate R2');
        Check('part next', Format(S.Resources.Next(), 0, 9) + '|' + S.Resources."No.".Value() + '|' + S.Resources."Group Code".Value(), 'false|R1|G1');
        Check('part next log', Log.Take(), 'agcr R1');
        Check('part first', Format(S.Resources.First(), 0, 9) + '|' + S.Resources."No.".Value(), 'true|R1');
        Check('part previous', Format(S.Resources.Previous(), 0, 9) + '|' + S.Resources."No.".Value(), 'false|R1');
        Check('part last', Format(S.Resources.Last(), 0, 9) + '|' + S.Resources."No.".Value(), 'true|R1');
        Check('part walk log', Log.Take(), '');
        Check('host', S.ResourceNo.Value() + '|' + S."No.".Value(), 'R2|R1');
        Check('next', Format(S.Next(), 0, 9) + '|' + S."No.".Value() + '|' + S.Resources."No.".Value() + '|' + S.ResourceNo.Value(), 'true|R2|R2|R2');
        Check('next log', Log.Take(), 'agcr R2');
        S.Close();
    end;
    // :213 Y15_ThreeLink_LookupShowsPickedResource.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure PartNextAfterReads()
    var S: TestPage "Repro Three Link Schedule"; Setting: Record "Repro Lookup Setting";
    begin
        Setup(); S.OpenEdit(); Log.Take();
        S.ResourceNo.Lookup();
        Check('resource no', S.ResourceNo.Value(), 'R2');
        Check('row', S."No.".Value(), 'R1');
        Check('part', S.Resources."No.".Value() + '/' + S.Resources."Group Code".Value(), 'R1/G1');
        Check('part next', Format(S.Resources.Next(), 0, 9), 'false');
        Setting.Get();
        Check('stored', Setting."Resource No.", 'R2');
        S.Close();
    end;
    // :272 Y15_ThreeLink_LookupCancelled.
    [Test] [HandlerFunctions('CancelLookup')]
    procedure LookupCancelled()
    var S: TestPage "Repro Three Link Schedule";
    begin
        Setup(); S.OpenEdit(); Log.Take();
        S.ResourceNo.Lookup();
        Check('lookup log', Log.Take(), 'lookup');
        Check('resource no', S.ResourceNo.Value(), '');
        Check('part', S.Resources."No.".Value(), 'R1');
        S.Close();
    end;
    // :297 Y15_ThreeLink_LookupTwice.
    [Test] [HandlerFunctions('PickSecondThenThird')]
    procedure LookupTwice()
    var S: TestPage "Repro Three Link Schedule";
    begin
        Setup(); PickCount := 0; S.OpenEdit(); Log.Take();
        S.ResourceNo.Lookup();
        Check('first', S.ResourceNo.Value(), 'R2');
        S.ResourceNo.Lookup();
        Check('second', S.ResourceNo.Value(), 'R3');
        Check('log', Log.Take(), 'lookup;validate R2;agcr R1;lookup;validate R3');
        S.Close();
    end;
    // :245 Y15_ThreeLink_RegionsSetPartFollowsRow: the host's Next while the refresh is pending; its trigger order is pinned
    // only on the hosts with function controls (:324 `next log`).
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure HostNextWhilePending()
    var S: TestPage "Repro Three Link Schedule";
    begin
        Setup(); S.OpenEdit(); S.ResourceNo.Lookup(); S.Next();
    end;
    local procedure Check(What: Text; Actual: Text; Expected: Text)
    begin if Actual <> Expected then Error('%1: [%2], expected [%3]', What, Actual, Expected); end;
    [ModalPageHandler]
    procedure PickSecondResource(var ResourceList: TestPage "Repro Lookup Resource List")
    begin ResourceList.GoToKey('R2'); ResourceList.OK().Invoke(); end;
    [ModalPageHandler]
    procedure CancelLookup(var ResourceList: TestPage "Repro Lookup Resource List")
    begin ResourceList.GoToKey('R2'); ResourceList.Cancel().Invoke(); end;
    [ModalPageHandler]
    procedure PickSecondThenThird(var ResourceList: TestPage "Repro Lookup Resource List")
    begin
        PickCount += 1;
        if PickCount = 1 then ResourceList.GoToKey('R2') else ResourceList.GoToKey('R3');
        ResourceList.OK().Invoke();
    end;
    local procedure Setup()
    var Setting: Record "Repro Lookup Setting"; Resource: Record "Repro Lookup Resource";
    begin
        Setting.DeleteAll(); Resource.DeleteAll();
        InsertResource(Resource, 'R1', 'G1'); InsertResource(Resource, 'R2', 'G1'); InsertResource(Resource, 'R3', 'G2');
    end;
    local procedure InsertResource(var Resource: Record "Repro Lookup Resource"; ResourceNo: Code[20]; GroupCode: Code[20])
    begin Resource.Init(); Resource."No." := ResourceNo; Resource."Group Code" := GroupCode; Resource.Insert(); end;
}
"#;

#[test]
fn the_first_part_move_after_a_lookup_runs_the_refresh_it_left() {
    let o = outcomes_in(SRC, "Repro Deferred Refresh Tests");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    for pass in [
        "PartWalkAfterLookup",
        "PartNextAfterReads",
        "LookupCancelled",
        "LookupTwice",
    ] {
        assert_eq!(get(pass), "Some(Pass)", "{pass}");
    }
    let g = get("HostNextWhilePending");
    assert!(
        g.contains("NotRun") && g.contains("BC runs the refresh of the OnValidate's CurrPage.Update(false) first"),
        "{g}"
    );
}
