//! TestField.Lookup through OnLookup, whose OnValidate calls `CurrPage.Update(false)`, on a host with OnAfterGetCurrRecord
//! whose ListPart has a SubPageLink of three `field(..)` entries shows the picked value (al-runner issue 6519, the reporter's
//! repro; real BC shows `R2`). BC leaves the refresh of the Update for later: the Lookup and the reads after it run no row
//! trigger, the globals the triggers set and the part keep their values (corpus writer Y26, branch al-runtime-rs-staging-y26,
//! pages/lt-y15/Y15LookupThreeFieldLink.al:314 Y26_ThreeLinkCurr_LookupRefresh). The calls that run that refresh are refused.

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
page 50105 "Repro Multi Link Schedule"
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
        repeater(Lines) { field("No."; Rec."No.") { } }
        part(Resources; "Repro Lookup Resource Part") { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"); }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    var Setting: Record "Repro Lookup Setting";
}
page 50109 "Repro Three Link Schedule"
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
        part(Resources; "Repro Lookup Resource Part")
        {
            SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code");
        }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin CurrentNo := Rec."No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting";
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
    layout { area(Content) { repeater(Lines) { field("No."; Rec."No.") { } field(PartCurrent; PartCurrent) { } field(PartFn; GetPartNo()) { } } } }
    trigger OnAfterGetRecord() begin Log.Add('p-agr ' + Rec."No."); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('p-agcr ' + Rec."No."); PartCurrent := Rec."No."; end;
    local procedure GetPartNo(): Code[20] begin Log.Add('p-fn ' + Rec."No."); exit(Rec."No."); end;
    var PartCurrent: Code[20]; Log: Codeunit "Repro Log";
}
page 50132 "Repro Logged Schedule"
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
        part(Resources; "Repro Logging Part")
        {
            SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code");
        }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetRecord() begin Log.Add('h-agr ' + Rec."No."); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('h-agcr ' + Rec."No."); CurrentNo := Rec."No." + '/' + Setting."Resource No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting"; Log: Codeunit "Repro Log";
}
page 50133 "Repro Host Fn Schedule"
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
        part(Resources; "Repro Lookup Resource Part")
        {
            SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code");
        }
    } }
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin end;
    local procedure GetHostNo(): Code[20] begin Log.Add('h-fn ' + Rec."No."); exit(Rec."No." + '/' + Setting."Resource No."); end;
    var Setting: Record "Repro Lookup Setting"; Log: Codeunit "Repro Log";
}
codeunit 50110 "Repro Lookup Link Tests"
{
    Subtype = Test;
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure TwoLinkedFieldsShowThePickedResource()
    var Schedule: TestPage "Repro Multi Link Schedule";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup();
        AssertPickedResource(Schedule.ResourceNo.Value()); Schedule.Close();
    end;
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure ThreeLinkedFieldsShowThePickedResource()
    var Schedule: TestPage "Repro Three Link Schedule";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup();
        AssertPickedResource(Schedule.ResourceNo.Value()); Schedule.Close();
    end;
    // Y15LookupThreeFieldLink.al:314 Y26_ThreeLinkCurr_LookupRefresh, the reads of a field, a global and the part's fields.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure LoggedLookupRunsNoRowTrigger()
    var S: TestPage "Repro Logged Schedule"; Log: Codeunit "Repro Log"; Setting: Record "Repro Lookup Setting";
    begin
        Setup(); Log.Take(); S.OpenEdit(); Log.Take();
        S.ResourceNo.Lookup();
        Check('lookup log', Log.Take(), 'lookup;validate R2');
        Check('resource no', S.ResourceNo.Value(), 'R2');
        Check('header current', S.HeaderCurrent.Value(), 'R1/');
        Check('current no', S.CurrentNo.Value(), 'R1/');
        Check('row', S."No.".Value(), 'R1');
        Check('part no', S.Resources."No.".Value(), 'R1');
        Check('part current', S.Resources.PartCurrent.Value(), 'R1');
        Check('reads log', Log.Take(), '');
        S.Close();
        Check('close log', Log.Take(), '');
        Setting.Get();
        Check('stored', Setting."Resource No.", 'R2');
    end;
    // :314 `next log`: Next runs the refresh first, which is not built.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure NextAfterTheLookup()
    var S: TestPage "Repro Logged Schedule";
    begin
        Setup(); S.OpenEdit(); S.ResourceNo.Lookup(); S.Next();
    end;
    // :314 `part fn` `R1/`: a function control of the part keeps what it showed before.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure PartFunctionAfterTheLookup()
    var S: TestPage "Repro Logged Schedule";
    begin
        Setup(); S.OpenEdit(); S.ResourceNo.Lookup(); S.Resources.PartFn.Value();
    end;
    // :314 `lookup log` `lookup;h-fn R1;validate R2;h-fn R1`: BC computes the host's function control within the Lookup.
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure HostFunctionControl()
    var S: TestPage "Repro Host Fn Schedule";
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
    local procedure AssertPickedResource(ActualResourceNo: Text)
    begin if ActualResourceNo <> 'R2' then Error('Resource No. shows [%1] after the lookup.', ActualResourceNo); end;
}
"#;

#[test]
fn a_lookup_on_a_host_with_row_triggers_whose_part_links_three_fields_shows_the_picked_value() {
    let o = outcomes_in(SRC, "Repro Lookup Link Tests");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    assert_eq!(get("TwoLinkedFieldsShowThePickedResource"), "Some(Pass)");
    assert_eq!(get("ThreeLinkedFieldsShowThePickedResource"), "Some(Pass)");
    assert_eq!(get("LoggedLookupRunsNoRowTrigger"), "Some(Pass)");
    for refused in ["NextAfterTheLookup", "PartFunctionAfterTheLookup"] {
        let g = get(refused);
        assert!(
            g.contains("NotRun") && g.contains("BC runs the refresh of the OnValidate's CurrPage.Update(false) first"),
            "{refused}: {g}"
        );
    }
    let g = get("HostFunctionControl");
    assert!(
        g.contains("NotRun") && g.contains("whose refresh of the page is not pinned"),
        "{g}"
    );
}

/// The reporter's tests run clean: the Lookup writes the field of the page's global record (`Setting."Resource No."`) and runs
/// the control's OnValidate, which the corpus pins (pages/lt-sc-outside-pages/ScLookupGlobalRecordField.al: the control shows
/// R2 and the saved setting row holds it), so assumption `page-expression-control-setvalue` is not marked.
#[test]
fn the_reporters_lookups_mark_no_assumption() {
    let l = al_testkit::compile(SRC);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "Repro Lookup Link Tests")
        .unwrap();
    let mut hits = Vec::new();
    let out = al_testrun::runtime::run_codeunit_full(
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
        "TwoLinkedFieldsShowThePickedResource",
        "ThreeLinkedFieldsShowThePickedResource",
    ] {
        let o = out.iter().find(|(t, _)| t == n).map(|(_, o)| format!("{o:?}"));
        assert_eq!(o.as_deref(), Some("Pass"), "{n}");
        let h = hits
            .iter()
            .find(|(t, _)| t == n)
            .map(|(_, h)| h.clone())
            .unwrap_or_default();
        assert!(h.is_empty(), "{n} marks {h:?}");
    }
}
