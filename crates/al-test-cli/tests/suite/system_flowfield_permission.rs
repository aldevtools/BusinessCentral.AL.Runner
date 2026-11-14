//! "Published Application"."Tenant Visible" looks up "NAV App Extra" (2000000157). Under a set that grants read on Published
//! Application and nothing on NAV App Extra, that read is not checked against the session's sets in two shapes: a Lookup
//! FlowField of an app table filtered on "Tenant Visible" = const(true) calculated by CalcFields, and FindFirst of Published
//! Application filtered on ID and "Tenant Visible" = true (al-runner issue 8038, assumption
//! `system-flowfield-reads-internal-table`). Every neighbor shape keeps main's outcome.
//!
//! Needs `System.app` of the pinned build in the artifact cache (`al_testfiles`).

use super::y48_platform_probes::outcomes_with_hits_app;
use al_builtins::assumptions::Assumption;
use al_test_cli::report::Outcome;

const APP_ID: &str = "8c5e3a1f-2b47-4d9e-a6c0-3f1e9d7b2a58";

const SRC: &str = r#"
dotnet { assembly("Microsoft.Dynamics.Nav.PermissionTestHelper") {
  type("Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper"; "PermissionTestHelper") { } } }
table 50600 "Repro Item"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Extension ID"; Guid) { }
        field(3; "Extension Name"; Text[250])
        { FieldClass = FlowField; CalcFormula = lookup("Published Application".Name where(ID = field("Extension ID"), "Tenant Visible" = const(true))); }
        field(4; "Extension Publisher"; Text[250])
        { FieldClass = FlowField; CalcFormula = lookup("Published Application".Publisher where(ID = field("Extension ID"), "Tenant Visible" = const(true))); }
        field(5; "Name Any"; Text[250])
        { FieldClass = FlowField; CalcFormula = lookup("Published Application".Name where(ID = field("Extension ID"))); }
        field(6; "Exists Visible"; Boolean)
        { FieldClass = FlowField; CalcFormula = exist("Published Application" where(ID = field("Extension ID"), "Tenant Visible" = const(true))); }
        field(7; "Name Hidden"; Text[250])
        { FieldClass = FlowField; CalcFormula = lookup("Published Application".Name where(ID = field("Extension ID"), "Tenant Visible" = const(false))); }
        field(8; "Name PerTenant"; Text[250])
        { FieldClass = FlowField; CalcFormula = lookup("Published Application".Name where(ID = field("Extension ID"), "PerTenant Or Installed" = const(true))); }
        field(10; "Count Visible"; Integer)
        { FieldClass = FlowField; CalcFormula = count("Published Application" where(ID = field("Extension ID"), "Tenant Visible" = const(true))); }
        field(11; "Id Filter"; Guid) { FieldClass = FlowFilter; }
        field(12; "Name FF"; Text[250])
        { FieldClass = FlowField; CalcFormula = lookup("Published Application".Name where(ID = field("Id Filter"), "Tenant Visible" = const(true))); }
        field(13; Data; Blob) { }
        field(9; "Name Visible Key"; Text[250])
        { FieldClass = FlowField; CalcFormula = lookup("Published Application".Name where("Package ID" = field("Extension ID"), "Tenant Visible" = const(true))); }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}
xmlport 50600 "Repro Export"
{
    Direction = Export;
    schema { textelement(Root) { tableelement(Item; "Repro Item") { fieldelement(Code; Item."Code") { } fieldelement(Name; Item."Extension Name") { } } } }
}
page 50600 "Repro Card"
{
    PageType = Card;
    SourceTable = "Repro Item";
    layout { area(Content) { field("Extension Name"; Rec."Extension Name") { } } }
}
query 50600 "Repro Q"
{
    elements { dataitem(Item; "Repro Item") { column(Code; "Code") { } column(Name; "Extension Name") { } } }
}
permissionset 50601 "PA READ" { Assignable = true; Permissions = tabledata "Repro Item" = RIMD, tabledata "Published Application" = R, codeunit T = X, codeunit Hlp = X, codeunit TX = X, codeunit TF = X, codeunit TP = X, codeunit TQ = X, xmlport "Repro Export" = X, page "Repro Card" = X, query "Repro Q" = X; }
permissionset 50602 "NO PA" { Assignable = true; Permissions = tabledata "Repro Item" = RIMD, codeunit T = X, codeunit Hlp = X, codeunit TX = X, codeunit TF = X, codeunit TP = X, codeunit TQ = X, xmlport "Repro Export" = X, page "Repro Card" = X, query "Repro Q" = X; }
permissionset 50603 "PA IND" { Assignable = true; Permissions = tabledata "Repro Item" = RIMD, tabledata "Published Application" = r, codeunit T = X, codeunit Hlp = X, codeunit TX = X, codeunit TF = X, codeunit TP = X, codeunit TQ = X, xmlport "Repro Export" = X, page "Repro Card" = X, query "Repro Q" = X; }
codeunit 50610 Hlp
{
    procedure Item(var Item: Record "Repro Item"; Found: Boolean; Role: Code[20])
    var
        H: DotNet PermissionTestHelper;
        Me: ModuleInfo;
    begin
        NavApp.GetCurrentModuleInfo(Me);
        Item."Code" := 'C';
        if Found then
            Item."Extension ID" := Me.Id
        else
            Item."Extension ID" := CreateGuid();
        H := H.PermissionTestHelper();
        H.AddEffectivePermissionSet(Role);
    end;
}
codeunit 50602 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Item(var Item: Record "Repro Item"; Found: Boolean; Role: Code[20])
    var
        H: DotNet PermissionTestHelper;
        Me: ModuleInfo;
    begin
        NavApp.GetCurrentModuleInfo(Me);
        Item."Code" := 'C';
        if Found then
            Item."Extension ID" := Me.Id
        else
            Item."Extension ID" := CreateGuid();
        H := H.PermissionTestHelper();
        H.AddEffectivePermissionSet(Role);
    end;

    [Test]
    procedure FoundName()
    var
        I: Record "Repro Item";
    begin
        Item(I, true, 'PA READ');
        I.CalcFields("Extension Name");
        if I."Extension Name" <> 'main' then
            Error('name %1', I."Extension Name");
    end;

    [Test]
    procedure FoundPublisher()
    var
        I: Record "Repro Item";
    begin
        Item(I, true, 'PA READ');
        I.CalcFields("Extension Publisher");
    end;

    [Test]
    procedure NotFoundName()
    var
        I: Record "Repro Item";
    begin
        Item(I, false, 'PA READ');
        I.CalcFields("Extension Name");
        if I."Extension Name" <> '' then
            Error('unexpected name');
    end;

    [Test]
    procedure FoundNameAny()
    var
        I: Record "Repro Item";
    begin
        Item(I, true, 'PA READ');
        I.CalcFields("Name Any");
    end;

    [Test]
    procedure FoundExistsVisible()
    var
        I: Record "Repro Item";
    begin
        Item(I, true, 'PA READ');
        I.CalcFields("Exists Visible");
        if not I."Exists Visible" then
            Error('exists');
    end;

    [Test]
    procedure FoundCountVisible()
    var
        I: Record "Repro Item";
    begin
        Item(I, true, 'PA READ');
        I.CalcFields("Count Visible");
        if I."Count Visible" <> 1 then
            Error('count %1', I."Count Visible");
    end;

    [Test]
    procedure FoundNameHidden()
    var
        I: Record "Repro Item";
    begin
        Item(I, true, 'PA READ');
        I.CalcFields("Name Hidden");
    end;

    [Test]
    procedure FoundNamePerTenant()
    var
        I: Record "Repro Item";
    begin
        Item(I, true, 'PA READ');
        I.CalcFields("Name PerTenant");
    end;

    [Test]
    procedure FoundNameOtherKey()
    var
        I: Record "Repro Item";
        Me: ModuleInfo;
    begin
        Item(I, true, 'PA READ');
        NavApp.GetCurrentModuleInfo(Me);
        I."Extension ID" := Me.PackageId;
        I.CalcFields("Name Visible Key");
    end;

    [Test]
    procedure FoundNameIndirect()
    var
        I: Record "Repro Item";
    begin
        Item(I, true, 'PA IND');
        I.CalcFields("Extension Name");
    end;

    [Test]
    procedure FoundNameNoPA()
    var
        I: Record "Repro Item";
    begin
        Item(I, true, 'NO PA');
        I.CalcFields("Extension Name");
    end;

    [Test]
    procedure DirectTenantVisible()
    var
        I: Record "Repro Item";
        PA: Record "Published Application";
        Me: ModuleInfo;
    begin
        Item(I, true, 'PA READ');
        NavApp.GetCurrentModuleInfo(Me);
        PA.SetRange(ID, Me.Id);
        PA.FindFirst();
        PA.CalcFields("Tenant Visible");
    end;

    local procedure Direct(var PA: Record "Published Application"; Role: Code[20])
    var
        I: Record "Repro Item";
        Me: ModuleInfo;
    begin
        Item(I, true, Role);
        NavApp.GetCurrentModuleInfo(Me);
        PA.SetRange(ID, Me.Id);
    end;

    [Test]
    procedure DirectFindFirst()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange("Tenant Visible", true);
        if not PA.FindFirst() then
            Error('not found');
        if PA.Name <> 'main' then
            Error('name %1', PA.Name);
    end;

    [Test]
    procedure DirectFindSet()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange("Tenant Visible", true);
        PA.FindSet();
    end;

    [Test]
    procedure DirectIsEmpty()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange("Tenant Visible", true);
        if PA.IsEmpty() then
            Error('empty');
    end;

    [Test]
    procedure DirectExtraFilter()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange("Tenant Visible", true);
        PA.SetRange(Name, 'main');
        PA.FindFirst();
    end;

    [Test]
    procedure DirectNoIdFilter()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange(ID);
        PA.SetRange("Tenant Visible", true);
        PA.FindFirst();
    end;

    [Test]
    procedure DirectVisibleFalse()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange("Tenant Visible", false);
        if PA.FindFirst() then;
    end;

    [Test]
    procedure DirectOtherGroup()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.FilterGroup(2);
        PA.SetRange("Tenant Visible", true);
        PA.FindFirst();
    end;

    [Test]
    procedure DirectIndirect()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA IND');
        PA.SetRange("Tenant Visible", true);
        if PA.FindFirst() then;
    end;

    [Test]
    procedure DirectDescending()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange("Tenant Visible", true);
        PA.Ascending(false);
        PA.FindFirst();
    end;

    [Test]
    procedure DirectCurrentKey()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange("Tenant Visible", true);
        PA.SetCurrentKey(Name);
        PA.FindFirst();
    end;

    [Test]
    procedure DirectLoadFields()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange("Tenant Visible", true);
        PA.SetLoadFields(Name);
        PA.FindFirst();
    end;

    [Test]
    procedure DirectAutoCalc()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange("Tenant Visible", true);
        PA.SetAutoCalcFields("Tenant Visible");
        PA.FindFirst();
    end;

    [Test]
    procedure DirectSecurityFiltering()
    var
        [SecurityFiltering(SecurityFilter::Filtered)]
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange("Tenant Visible", true);
        PA.FindFirst();
    end;

    [Test]
    procedure DirectReadIsolation()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange("Tenant Visible", true);
        PA.ReadIsolation := IsolationLevel::ReadUncommitted;
        PA.FindFirst();
    end;

    [Test]
    procedure DirectCurrentGroup()
    var
        PA: Record "Published Application";
    begin
        Direct(PA, 'PA READ');
        PA.SetRange("Tenant Visible", true);
        PA.FilterGroup(2);
        PA.FindFirst();
    end;

    [Test]
    procedure DirectRecordRef()
    var
        I: Record "Repro Item";
        RR: RecordRef;
        FR: FieldRef;
        Me: ModuleInfo;
    begin
        Item(I, true, 'PA READ');
        NavApp.GetCurrentModuleInfo(Me);
        RR.Open(Database::"Published Application");
        FR := RR.Field(3);
        FR.SetRange(Me.Id);
        FR := RR.Field(30);
        FR.SetRange(true);
        RR.FindFirst();
    end;

    [Test]
    procedure FoundNameFlowFilterId()
    var
        I: Record "Repro Item";
        Me: ModuleInfo;
    begin
        Item(I, true, 'PA READ');
        NavApp.GetCurrentModuleInfo(Me);
        I.SetRange("Id Filter", Me.Id);
        I.CalcFields("Name FF");
    end;

    [Test]
    procedure FoundNameFlowFilterNone()
    var
        I: Record "Repro Item";
    begin
        Item(I, true, 'PA READ');
        I.CalcFields("Name FF");
    end;

    [Test]
    procedure FoundNameTemporary()
    var
        I: Record "Repro Item" temporary;
    begin
        Item(I, true, 'PA READ');
        I.CalcFields("Extension Name");
    end;

    // The only test that writes: last, so its refusal cannot hold back another test.
    [Test]
    procedure FoundNameFilter()
    var
        I: Record "Repro Item";
    begin
        Item(I, true, 'PA READ');
        I.Insert();
        I.SetFilter("Extension Name", '<>%1', '');
        if I.Count() <> 1 then
            Error('count');
    end;
}
codeunit 50603 TX
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Hlp: Codeunit Hlp;

    [Test]
    procedure FoundNameXmlPort()
    var
        I: Record "Repro Item";
        Tmp: Record "Repro Item" temporary;
        Os: OutStream;
    begin
        Hlp.Item(I, true, 'PA READ');
        I.Insert();
        Tmp.Data.CreateOutStream(Os);
        Xmlport.Export(Xmlport::"Repro Export", Os, I);
    end;
}
codeunit 50604 TF
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Hlp: Codeunit Hlp;

    [Test]
    procedure FoundNameFieldRef()
    var
        I: Record "Repro Item";
        RR: RecordRef;
        FR: FieldRef;
    begin
        Hlp.Item(I, true, 'PA READ');
        I.Insert();
        RR.Open(Database::"Repro Item");
        RR.Get(I.RecordId);
        FR := RR.Field(3);
        FR.CalcField();
    end;
}
codeunit 50605 TP
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Hlp: Codeunit Hlp;

    [Test]
    procedure FoundNamePageControl()
    var
        I: Record "Repro Item";
        P: TestPage "Repro Card";
    begin
        Hlp.Item(I, true, 'PA READ');
        I.Insert();
        P.OpenView();
        P.GoToRecord(I);
        if P."Extension Name".Value = 'x' then;
    end;
}
codeunit 50606 TQ
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Hlp: Codeunit Hlp;

    [Test]
    procedure FoundNameQuery()
    var
        I: Record "Repro Item";
        Q: Query "Repro Q";
    begin
        Hlp.Item(I, true, 'PA READ');
        I.Insert();
        Q.Open();
        if Q.Read() then;
    end;
}
"#;

fn denial_on_nav_app_extra(why: &str) -> bool {
    why.contains("object type 0 id 2000000157") && why.contains("a FlowField over a system table")
}

#[test]
fn tenant_visible_lookup_reads_nav_app_extra_unchecked_neighbors_keep_main() {
    let mut o = outcomes_with_hits_app(APP_ID, SRC, "T");
    // Each test that writes has a codeunit of its own: a refusal holds back the rest of its codeunit.
    for cu in ["TX", "TF", "TP", "TQ"] {
        o.extend(outcomes_with_hits_app(APP_ID, SRC, cu));
    }
    let get = |name: &str| {
        o.iter()
            .find(|(n, _, _)| n == name)
            .unwrap_or_else(|| panic!("{name} in {o:?}"))
    };
    let flag = Assumption::SystemFlowfieldReadsInternalTable;
    // The reporter's shapes, Guided Experience's (Lookup of Name and of Publisher through CalcFields; FindFirst of
    // Published Application filtered on ID and "Tenant Visible") and the probe's Exist and Count: pass, flagged.
    for name in [
        "FoundName",
        "FoundPublisher",
        "FoundExistsVisible",
        "FoundCountVisible",
        "DirectFindFirst",
    ] {
        let (_, outcome, hits) = get(name);
        assert_eq!(*outcome, Outcome::Pass, "{name}");
        assert!(hits.contains(&flag), "{name}: {hits:?}");
    }
    // Main passes these: no row reaches the condition FlowField, or no condition on it. No flag.
    for name in ["NotFoundName", "FoundNameAny"] {
        let (_, outcome, hits) = get(name);
        assert_eq!(*outcome, Outcome::Pass, "{name}");
        assert!(!hits.contains(&flag), "{name}: {hits:?}");
    }
    // Neighbors the evidence does not measure keep main's refusal on NAV App Extra: another constant,
    // the other FlowField over NAV App Extra, another key condition, a filter on the FlowField, an indirect grant on
    // Published Application, and CalcFields of "Tenant Visible" itself; for the direct read: FindSet, IsEmpty, another
    // filter, no ID filter, the constant false, another filter group.
    for name in [
        "DirectFindSet",
        "DirectIsEmpty",
        "DirectExtraFilter",
        "DirectNoIdFilter",
        "DirectVisibleFalse",
        "DirectOtherGroup",
        "FoundNameHidden",
        "FoundNamePerTenant",
        "FoundNameOtherKey",
        "FoundNameFilter",
        "FoundNameIndirect",
        "DirectTenantVisible",
        "DirectDescending",
        "DirectCurrentKey",
        "DirectLoadFields",
        "DirectAutoCalc",
        "DirectSecurityFiltering",
        "DirectReadIsolation",
        "DirectCurrentGroup",
        "DirectRecordRef",
        "FoundNameFlowFilterId",
        "FoundNameFlowFilterNone",
        "FoundNameTemporary",
        "FoundNameXmlPort",
        "FoundNameFieldRef",
    ] {
        match &get(name).1 {
            Outcome::NotRun(why) => assert!(denial_on_nav_app_extra(why), "{name}: {why}"),
            other => panic!("{name}: {other:?} {:?}", get(name).2),
        }
    }
    // A page control and a query over the FlowField: refused for the session's denial, never through the new path.
    for name in ["FoundNamePageControl", "FoundNameQuery"] {
        let (_, outcome, hits) = get(name);
        assert_ne!(*outcome, Outcome::Pass, "{name}");
        assert!(!hits.contains(&flag), "{name}: {hits:?}");
    }
    // Without a grant on Published Application, or with an indirect one, the read of Published Application is still
    // checked: main's outcome, never the new path.
    for name in ["FoundNameNoPA", "DirectIndirect"] {
        let (_, outcome, hits) = get(name);
        assert_ne!(*outcome, Outcome::Pass, "{name}");
        assert!(!hits.contains(&flag), "{name}: {hits:?}");
    }
}
