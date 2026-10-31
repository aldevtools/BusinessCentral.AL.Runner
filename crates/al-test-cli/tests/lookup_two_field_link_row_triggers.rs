//! TestField.Lookup through OnLookup, whose OnValidate calls `CurrPage.Update(false)`, on a host with OnAfterGetRecord or
//! OnAfterGetCurrRecord whose ListPart has a SubPageLink of two `field(..)` entries: BC leaves the refresh of the Update for
//! later, as with three entries. The Lookup and the reads after it run no row trigger; the globals the triggers set and the
//! part keep their values (corpus writer Y26, branch al-runtime-rs-staging-y26, pages/lt-y15/Y15LookupThreeFieldLink.al:379
//! Y26_TwoLinkCurr_LookupRefresh, :509 Y26_TwoLinkPlain_LookupRefresh; al-runner issue 6519). The calls that run that
//! refresh are refused, and so is a host with row triggers whose part links one field (not measured).

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
codeunit 50130 "Repro Log"
{
    SingleInstance = true;
    procedure Add(E: Text) begin if Entries <> '' then Entries += ';'; Entries += E; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
    var Entries: Text;
}
page 50131 "Repro Logging Part"
{
    PageType = ListPart; SourceTable = "Repro Lookup Resource"; Editable = false;
    layout { area(Content) { repeater(Lines) { field("No."; Rec."No.") { } field("Group Code"; Rec."Group Code") { } field(PartCurrent; PartCurrent) { } field(PartFn; GetPartNo()) { } } } }
    trigger OnAfterGetRecord() begin Log.Add('p-agr ' + Rec."No."); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('p-agcr ' + Rec."No."); PartCurrent := Rec."No." + '/' + StoredResourceNo(); end;
    local procedure GetPartNo(): Code[20] begin Log.Add('p-fn ' + Rec."No."); exit(Rec."No." + '/' + StoredResourceNo()); end;
    local procedure StoredResourceNo(): Code[20] var Setting: Record "Repro Lookup Setting"; begin if Setting.Get() then exit(Setting."Resource No."); end;
    var PartCurrent: Code[20]; Log: Codeunit "Repro Log";
}
// Y15LookupThreeFieldLink.al page 92222 "LT Y26 2Link Curr Schedule", without the function control HostFn.
page 50132 "Repro Two Link Curr Schedule"
{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None;
    layout { area(Content) {
        group(Filters) {
            field(ResourceNo; Setting."Resource No.") {
                trigger OnLookup(var Text: Text): Boolean
                var Resource: Record "Repro Lookup Resource";
                begin
                    Log.Add('lookup');
                    if Page.RunModal(Page::"Repro Lookup Resource List", Resource) <> Action::LookupOK then exit(false);
                    Text := Resource."No."; exit(true);
                end;
                trigger OnValidate() begin Log.Add('validate ' + Setting."Resource No."); Setting.Modify(); CurrPage.Update(false); end;
            }
            field(HeaderCurrent; CurrentNo) { }
        }
        repeater(Lines) { field("No."; Rec."No.") { } field(CurrentNo; CurrentNo) { } }
        part(Resources; "Repro Logging Part") { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"); }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetRecord() begin Log.Add('h-agr ' + Rec."No."); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('h-agcr ' + Rec."No."); CurrentNo := Rec."No." + '/' + Setting."Resource No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting"; Log: Codeunit "Repro Log";
}
// Page 92224 "LT Y26 2Link Plain Schedule", without HostFn: CurrentNo is set in OnAfterGetRecord.
page 50133 "Repro Two Link Plain Schedule"
{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None;
    layout { area(Content) {
        group(Filters) {
            field(ResourceNo; Setting."Resource No.") {
                trigger OnLookup(var Text: Text): Boolean
                var Resource: Record "Repro Lookup Resource";
                begin
                    Log.Add('lookup');
                    if Page.RunModal(Page::"Repro Lookup Resource List", Resource) <> Action::LookupOK then exit(false);
                    Text := Resource."No."; exit(true);
                end;
                trigger OnValidate() begin Log.Add('validate ' + Setting."Resource No."); Setting.Modify(); CurrPage.Update(false); end;
            }
            field(HeaderCurrent; CurrentNo) { }
        }
        repeater(Lines) { field("No."; Rec."No.") { } field(CurrentNo; CurrentNo) { } }
        part(Resources; "Repro Logging Part") { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"); }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetRecord() begin Log.Add('h-agr ' + Rec."No."); CurrentNo := Rec."No." + '/' + Setting."Resource No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting"; Log: Codeunit "Repro Log";
}
// The Y26 host with its function control: BC computes it within the Lookup (`h-fn`), this runtime does not.
page 50134 "Repro Two Link Host Fn"
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
        repeater(Lines) { field("No."; Rec."No.") { } field(HostFn; GetHostNo()) { } }
        part(Resources; "Repro Logging Part") { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"); }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin end;
    local procedure GetHostNo(): Code[20] begin exit(Rec."No." + '/' + Setting."Resource No."); end;
    var Setting: Record "Repro Lookup Setting";
}
// One link entry and a row trigger on the host: not measured.
page 50135 "Repro One Link Curr Schedule"
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
        part(Resources; "Repro Logging Part") { SubPageLink = "No." = field("No."); }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin CurrentNo := Rec."No." + '/' + Setting."Resource No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting";
}
codeunit 50110 "Repro Lookup Link Tests"
{
    Subtype = Test;
    // Y15LookupThreeFieldLink.al:379 Y26_TwoLinkCurr_LookupRefresh: `lookup log` (without h-fn), every read log empty, the
    // globals and the part as before.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure TwoLinkCurrLookupRunsNoRowTrigger()
    var S: TestPage "Repro Two Link Curr Schedule"; Log: Codeunit "Repro Log"; Setting: Record "Repro Lookup Setting";
    begin
        Setup(); S.OpenEdit(); Log.Take();
        S.ResourceNo.Lookup();
        Check('lookup log', Log.Take(), 'lookup;validate R2');
        Check('resource no', S.ResourceNo.Value(), 'R2');
        Check('header current', S.HeaderCurrent.Value(), 'R1/');
        Check('current no', S.CurrentNo.Value(), 'R1/');
        Check('row', S."No.".Value(), 'R1');
        Check('part no', S.Resources."No.".Value(), 'R1');
        Check('part current', S.Resources.PartCurrent.Value(), 'R1/');
        Check('reads log', Log.Take(), '');
        S.Close();
        Check('close log', Log.Take(), '');
        Setting.Get();
        Check('stored', Setting."Resource No.", 'R2');
    end;
    // :509 Y26_TwoLinkPlain_LookupRefresh: the same without OnAfterGetCurrRecord on the host.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure TwoLinkPlainLookupRunsNoRowTrigger()
    var S: TestPage "Repro Two Link Plain Schedule"; Log: Codeunit "Repro Log"; Setting: Record "Repro Lookup Setting";
    begin
        Setup(); S.OpenEdit(); Log.Take();
        S.ResourceNo.Lookup();
        Check('lookup log', Log.Take(), 'lookup;validate R2');
        Check('resource no', S.ResourceNo.Value(), 'R2');
        Check('header current', S.HeaderCurrent.Value(), 'R1/');
        Check('current no', S.CurrentNo.Value(), 'R1/');
        Check('part no', S.Resources."No.".Value(), 'R1');
        Check('part current', S.Resources.PartCurrent.Value(), 'R1/');
        Check('reads log', Log.Take(), '');
        S.Close();
        Check('close log', Log.Take(), '');
        Setting.Get();
        Check('stored', Setting."Resource No.", 'R2');
    end;
    // :379 `next log`: Next runs the refresh first, which is not built.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure NextAfterTheLookup()
    var S: TestPage "Repro Two Link Curr Schedule";
    begin
        Setup(); S.OpenEdit(); S.ResourceNo.Lookup(); S.Next();
    end;
    // :379 `part fn` `R1/`: a function control of the part keeps what it showed before.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure PartFunctionAfterTheLookup()
    var S: TestPage "Repro Two Link Curr Schedule";
    begin
        Setup(); S.OpenEdit(); S.ResourceNo.Lookup(); S.Resources.PartFn.Value();
    end;
    // :379 `lookup log` `lookup;h-fn R1;validate R2;h-fn R1`: BC computes the host's function control within the Lookup.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure HostFunctionControl()
    var S: TestPage "Repro Two Link Host Fn";
    begin
        Setup(); S.OpenEdit(); S.ResourceNo.Lookup();
        if S.ResourceNo.Value() <> 'R2' then Error('shows [%1]', S.ResourceNo.Value());
    end;
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure OneLinkWithRowTrigger()
    var S: TestPage "Repro One Link Curr Schedule";
    begin
        Setup(); S.OpenEdit(); S.ResourceNo.Lookup();
        if S.ResourceNo.Value() <> 'R2' then Error('shows [%1]', S.ResourceNo.Value());
    end;
    local procedure Check(What: Text; Actual: Text; Expected: Text)
    begin if Actual <> Expected then Error('%1: [%2], expected [%3]', What, Actual, Expected); end;
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
}
"#;

#[test]
fn a_lookup_on_a_host_with_row_triggers_whose_part_links_two_fields_leaves_the_refresh_for_later() {
    let o = outcomes_in(SRC, "Repro Lookup Link Tests");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    assert_eq!(get("TwoLinkCurrLookupRunsNoRowTrigger"), "Some(Pass)");
    assert_eq!(get("TwoLinkPlainLookupRunsNoRowTrigger"), "Some(Pass)");
    for refused in ["NextAfterTheLookup", "PartFunctionAfterTheLookup"] {
        let g = get(refused);
        assert!(
            g.contains("NotRun") && g.contains("BC runs the refresh of the OnValidate's CurrPage.Update(false) first"),
            "{refused}: {g}"
        );
    }
    for refused in ["HostFunctionControl", "OneLinkWithRowTrigger"] {
        let g = get(refused);
        assert!(
            g.contains("NotRun") && g.contains("whose refresh of the page is not pinned"),
            "{refused}: {g}"
        );
    }
}
