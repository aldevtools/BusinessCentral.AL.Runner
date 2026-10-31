//! ADR-062 stage 1: TableData writes of a session whose permissions a test lowered through `Permissions Mock`.
//!
//! Each test below mirrors one probe of corpus `tests/al-language/permissions/lt-y19-perm-probes/` (corpus master, PR
//! 914; BC 27.5, 28.5 and 29.0 agree) or an older pin. A denial in a pinned shape raises BC's error; an access the sets
//! allow runs; a denial in a shape no probe covers refuses the test (ADR-062 section 2, the safety rule).

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};
use std::sync::Arc;

const BASE: &str = r#"
dotnet
{
    assembly("Microsoft.Dynamics.Nav.PermissionTestHelper")
    {
        type("Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper"; "PermissionTestHelper")
        {
        }
    }
}

codeunit 131006 "Permissions Mock"
{
    SingleInstance = true;
    var
        PermissionTestHelper: DotNet PermissionTestHelper;
        Started: Boolean;
    procedure Start()
    begin
        PermissionTestHelper := PermissionTestHelper.PermissionTestHelper();
        Started := true;
    end;
    procedure Stop()
    begin
        if not Started then
            exit;
        Started := false;
        PermissionTestHelper.Dispose();
    end;
    procedure Assign(RoleID: Code[20])
    begin
        if not Started then
            exit;
        PermissionTestHelper.AddEffectivePermissionSet(RoleID);
    end;
    procedure Set(RoleID: Code[20])
    begin
        if not Started then
            exit;
        PermissionTestHelper.Clear();
        Assign('All Objects');
        Assign(RoleID);
    end;
    procedure SetExactPermissionSet(RoleID: Code[20])
    begin
        if not Started then
            exit;
        PermissionTestHelper.Clear();
        Assign(RoleID);
    end;
    procedure ClearAssignments()
    begin
        if not Started then
            exit;
        PermissionTestHelper.Clear();
    end;
}

permissionset 131006 "All Objects"
{
    Assignable = true;
    Permissions = table * = X, report * = X, codeunit * = X, page * = X, xmlport * = X, query * = X;
}

table 50980 "PB Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Value; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// Code of another app than the test's: the app name of a denial is the app whose code makes the access (Y19PermCrossApp.al).
codeunit 50980 "PB Writer"
{
    procedure InsertRow(NewId: Integer)
    var
        Row: Record "PB Row";
    begin
        Row.Id := NewId;
        Row.Insert();
    end;
}
"#;

const MAIN: &str = r#"
table 50990 "PX Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Value; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }

    procedure InsertSelf(NewId: Integer)
    var
        Row: Record "PX Row";
    begin
        Row.Id := NewId;
        Row.Insert();
    end;
}

table 50991 "PX Trig A"
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
    trigger OnInsert()
    var
        B: Record "PX Trig B";
    begin
        B.Id := Id + 100;
        B.Insert();
    end;
}

table 50992 "PX Trig A Perm"
{
    Permissions = tabledata "PX Trig B" = imd;
    fields
    {
        field(1; Id; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
    trigger OnInsert()
    var
        B: Record "PX Trig B";
    begin
        B.Id := Id + 100;
        B.Insert();
    end;
}

table 50993 "PX Trig B"
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 50994 "PX Inherent"
{
    InherentPermissions = RIMD;
    fields
    {
        field(1; Id; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 50995 "PX Ind Inherent"
{
    InherentPermissions = rimd;
    fields
    {
        field(1; Id; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

codeunit 50995 "PX Inherent Exec"
{
    InherentPermissions = X;
    Permissions = tabledata "PX Ind Inherent" = ri;
    procedure InsertRow(NewId: Integer)
    var
        Row: Record "PX Ind Inherent";
    begin
        Row.Id := NewId;
        Row.Insert();
    end;
}

codeunit 50990 "PX Deleter"
{
    Permissions = tabledata "PX Row" = d;
    procedure DeleteRow(RowId: Integer)
    var
        Row: Record "PX Row";
    begin
        Row.Id := RowId;
        Row.Delete();
    end;
}

interface "PX Inserter"
{
    procedure InsertRow(NewId: Integer);
}

codeunit 50991 "PX Frame Writer"
{
    Permissions = tabledata "PX Row" = ri;
    procedure InsertDirect(NewId: Integer)
    var
        Row: Record "PX Row";
    begin
        Row.Id := NewId;
        Row.Insert();
    end;
    procedure InsertViaLocal(NewId: Integer)
    begin
        LocalInsert(NewId);
    end;
    procedure InsertViaInterface(NewId: Integer)
    var
        Impl: Codeunit "PX Frame Impl";
        Inserter: Interface "PX Inserter";
    begin
        Inserter := Impl;
        Inserter.InsertRow(NewId);
    end;
    procedure InsertViaTableProc(NewId: Integer)
    var
        Row: Record "PX Row";
    begin
        Row.InsertSelf(NewId);
    end;
    local procedure LocalInsert(NewId: Integer)
    var
        Row: Record "PX Row";
    begin
        Row.Id := NewId;
        Row.Insert();
    end;
}

codeunit 50992 "PX Frame Impl" implements "PX Inserter"
{
    procedure InsertRow(NewId: Integer)
    var
        Row: Record "PX Row";
    begin
        Row.Id := NewId;
        Row.Insert();
    end;
}

permissionset 50990 "PX R"
{
    Assignable = true;
    Permissions = tabledata "PX Row" = R;
}

permissionset 50991 "PX RIM"
{
    Assignable = true;
    Permissions = tabledata "PX Row" = RIM;
}

permissionset 50992 "PX None"
{
    Assignable = true;
    Permissions = tabledata "PX Trig B" = R;
}

permissionset 50993 "PX Trig No B"
{
    Assignable = true;
    Permissions = tabledata "PX Trig A" = RIMD, tabledata "PX Trig A Perm" = RIMD;
}

permissionset 50994 "PX Trig Imd B"
{
    Assignable = true;
    Permissions = tabledata "PX Trig A" = RIMD, tabledata "PX Trig A Perm" = RIMD, tabledata "PX Trig B" = imd;
}

permissionset 50995 "PX R Ind D"
{
    Assignable = true;
    Permissions = tabledata "PX Row" = Rd;
}

permissionset 50996 "PX Ind RI"
{
    Assignable = true;
    Permissions = tabledata "PX Row" = ri;
}

permissionset 50997 "PX Ext Base"
{
    Assignable = true;
    Permissions = tabledata "PX Row" = R;
}

permissionsetextension 50998 "PX Ext Base Ext" extends "PX Ext Base"
{
    Permissions = tabledata "PX Row" = RIMD;
}

permissionset 50999 "PX Ind RIMD"
{
    Assignable = true;
    Permissions = tabledata "PX Row" = rimd;
}

permissionset 50989 "PX Excl Indirect"
{
    Assignable = true;
    Permissions = tabledata "PX Row" = RIMD;
    ExcludedPermissionSets = "PX Ind RIMD";
}

permissionset 50988 "PX Wild R"
{
    Assignable = true;
    Permissions = tabledata * = R;
}

permissionset 50987 "PX Base Row"
{
    Assignable = true;
    Permissions = tabledata "PB Row" = R;
}

codeunit 50993 "PX Perm Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";

    local procedure Seed()
    var
        Row: Record "PX Row";
    begin
        Mock.ClearAssignments();
        Row.DeleteAll();
        Row.Id := 1;
        Row.Value := 10;
        Row.Insert();
        Row.Id := 2;
        Row.Value := 20;
        Row.Insert();
    end;

    local procedure Lower(SetName: Code[20])
    begin
        Mock.Start();
        Seed();
        Mock.Set(SetName);
    end;

    local procedure Denied(What: Text; Op: Text; Code: Text)
    begin
        if GetLastErrorText() <> 'Sorry, the current permissions prevented the action. (' + What + ' ' + Op + ': main)' then
            Error('text: %1', GetLastErrorText());
        if GetLastErrorCode() <> Code then
            Error('code: %1', GetLastErrorCode());
    end;

    // TestMockBasics.al:72-73; Y19PermComposition.al:55 (test_direct_insert).
    [Test]
    procedure DirectInsertUnderR()
    var
        Row: Record "PX Row";
    begin
        Lower('PX R');
        Row.Id := 5;
        asserterror Row.Insert();
        Denied('TableData 50990 PX Row', 'Insert', 'DB:ClientInsertDenied');
        Mock.Stop();
    end;

    // Y19PermWrites.al Y19_P1_Rename: R gives the Modify error, RIM renames.
    [Test]
    procedure RenameUnderR()
    var
        Row: Record "PX Row";
    begin
        Lower('PX R');
        Row.Id := 1;
        Row.Value := 10;
        asserterror Row.Rename(11);
        Denied('TableData 50990 PX Row', 'Modify', 'DB:ClientModifyDenied');
        Mock.Stop();
    end;

    [Test]
    procedure RenameUnderRim()
    var
        Row: Record "PX Row";
    begin
        Lower('PX RIM');
        Row.Id := 1;
        Row.Value := 10;
        Row.Rename(11);
        Mock.Stop();
        if not Row.Get(11) then
            Error('not renamed');
    end;

    // Y19PermWrites.al Y19_P2_SetR: DeleteAll and ModifyAll fail on a non-empty table, also when the filter matches no row.
    [Test]
    procedure DeleteAllAndModifyAllUnderR()
    var
        Row: Record "PX Row";
    begin
        Lower('PX R');
        asserterror Row.DeleteAll();
        Denied('TableData 50990 PX Row', 'Delete', 'DB:ClientDeleteDenied');
        asserterror Row.ModifyAll(Value, 99, true);
        Denied('TableData 50990 PX Row', 'Modify', 'DB:ClientModifyDenied');
        Row.SetRange(Id, 100);
        asserterror Row.DeleteAll(true);
        Denied('TableData 50990 PX Row', 'Delete', 'DB:ClientDeleteDenied');
        Mock.Stop();
    end;

    // TestMockBasics.al:47-50: on an empty table DeleteAll and ModifyAll raise nothing.
    [Test]
    procedure DeleteAllOnAnEmptyTableUnderR()
    var
        Row: Record "PX Row";
    begin
        Mock.Start();
        Mock.ClearAssignments();
        Row.DeleteAll();
        Mock.Set('PX R');
        Row.DeleteAll();
        Row.ModifyAll(Value, 5);
        Mock.Stop();
    end;

    // Y19PermWrites.al Y19_P3_ImdOnB, Y19_P3_NothingOnB: a trigger's write is checked against the triggering table's
    // Permissions property.
    [Test]
    procedure TriggerWrites()
    var
        A: Record "PX Trig A";
        APerm: Record "PX Trig A Perm";
    begin
        Lower('PX Trig Imd B');
        A.Id := 1;
        asserterror A.Insert(true);
        Denied('TableData 50993 PX Trig B', 'Insert', 'DB:ClientInsertDenied');
        APerm.Id := 1;
        APerm.Insert(true);
        Mock.Set('PX Trig No B');
        APerm.Id := 2;
        asserterror APerm.Insert(true);
        Denied('TableData 50993 PX Trig B', 'IndirectInsert', 'DB:ClientInsertDenied');
        Mock.Stop();
    end;

    // Y19PermWrites.al Y19_P4_IndirectDeleteAndModify.
    [Test]
    procedure IndirectDelete()
    var
        Row: Record "PX Row";
        Deleter: Codeunit "PX Deleter";
    begin
        Lower('PX R Ind D');
        Deleter.DeleteRow(1);
        Row.Id := 2;
        asserterror Row.Delete();
        Denied('TableData 50990 PX Row', 'Delete', 'DB:ClientDeleteDenied');
        // The asserterror rolled the seed back (TestMockBasics.al:65).
        Seed();
        Mock.Set('PX None');
        asserterror Deleter.DeleteRow(2);
        Denied('TableData 50990 PX Row', 'IndirectDelete', 'DB:ClientDeleteDenied');
        Mock.Stop();
    end;

    // Y19PermWrites.al Y19_P6_RecordRef.
    [Test]
    procedure RecordRefInsert()
    var
        RecRef: RecordRef;
    begin
        Lower('PX None');
        RecRef.Open(Database::"PX Row");
        RecRef.Field(1).Value := 5;
        asserterror RecRef.Insert();
        Denied('TableData 50990 PX Row', 'Insert', 'DB:ClientInsertDenied');
        Mock.Stop();
    end;

    // Y19PermWrites.al Y19_P6_Temporary: a temporary record is never checked, its Count included (op 64-70).
    [Test]
    procedure TemporaryWrites()
    var
        TempRow: Record "PX Row" temporary;
    begin
        Lower('PX None');
        TempRow.Id := 5;
        TempRow.Insert();
        TempRow.Value := 3;
        TempRow.Modify();
        TempRow.Delete();
        if TempRow.Count() <> 0 then
            Error('count');
        Mock.Stop();
    end;

    // Y19PermComposition.al:45-58 (P7): the Permissions property covers the object's own and local procedures, not an
    // interface implementation in another codeunit nor a table procedure.
    [Test]
    procedure FrameRule()
    var
        Writer: Codeunit "PX Frame Writer";
    begin
        Lower('PX Ind RI');
        Writer.InsertDirect(5);
        Writer.InsertViaLocal(6);
        asserterror Writer.InsertViaInterface(7);
        Denied('TableData 50990 PX Row', 'Insert', 'DB:ClientInsertDenied');
        asserterror Writer.InsertViaTableProc(8);
        Denied('TableData 50990 PX Row', 'Insert', 'DB:ClientInsertDenied');
        Mock.Stop();
    end;

    // Y19PermComposition.al:100-101 (P10): a permission set extension grants at run time.
    [Test]
    procedure ExtensionGrants()
    var
        Row: Record "PX Row";
    begin
        Lower('PX Ext Base');
        Row.Id := 5;
        Row.Insert();
        Mock.Stop();
    end;

    // Y19PermComposition.al:159-162 (P11): RIMD excluding rimd leaves rimd.
    [Test]
    procedure ExcludingIndirectLeavesIndirect()
    var
        Row: Record "PX Row";
        Writer: Codeunit "PX Frame Writer";
    begin
        Lower('PX Excl Indirect');
        Row.Id := 5;
        asserterror Row.Insert();
        Denied('TableData 50990 PX Row', 'Insert', 'DB:ClientInsertDenied');
        Writer.InsertDirect(6);
        Mock.Stop();
    end;

    // Y19PermComposition.al:181-182 (P12).
    [Test]
    procedure WildcardRead()
    var
        Row: Record "PX Row";
    begin
        Lower('PX Wild R');
        if Row.Count() <> 2 then
            Error('count');
        Row.Id := 5;
        asserterror Row.Insert();
        Denied('TableData 50990 PX Row', 'Insert', 'DB:ClientInsertDenied');
        Mock.Stop();
    end;

    // Y19PermCrossApp.al (P5): the app name is the app whose code makes the access.
    [Test]
    procedure AppOfTheCode()
    var
        Writer: Codeunit "PB Writer";
    begin
        Lower('PX Base Row');
        asserterror Writer.InsertRow(1);
        if GetLastErrorText() <> 'Sorry, the current permissions prevented the action. (TableData 50980 PB Row Insert: base)' then
            Error('text: %1', GetLastErrorText());
        Mock.Stop();
    end;

    // RiskyInherentPerms.al:53-122 (pin 5): InherentPermissions = RIMD on the table grants.
    [Test]
    procedure TableInherentPermissions()
    var
        Inherent: Record "PX Inherent";
    begin
        Lower('PX None');
        Inherent.Id := 1;
        Inherent.Insert();
        Mock.Stop();
    end;

    // Rule 3 exception, assumption `inherent-permissions-indirect-and-objects` (Business Foundation Tests 134530): a
    // codeunit with InherentPermissions = X runs under a set without Execute grants, and its Permissions property
    // completes a table's indirect InherentPermissions.
    [Test]
    procedure ObjectAndIndirectInherentPermissions()
    var
        Exec: Codeunit "PX Inherent Exec";
    begin
        Mock.Start();
        Seed();
        Mock.SetExactPermissionSet('PX None');
        Exec.InsertRow(1);
        // No call of the mock after: the set grants no Execute on it.
    end;

    // An allowed read and a test that ends with its sets still assigned: it runs.
    [Test]
    procedure AllowedReadsRun()
    var
        Row: Record "PX Row";
    begin
        Lower('PX R');
        Row.FindFirst();
        if Row.Value <> 10 then
            Error('value');
    end;

    // A denied read is stage 2 (probes P14-P19): not run.
    [Test]
    procedure DeniedReadIsRefused()
    var
        Row: Record "PX Row";
    begin
        Lower('PX None');
        Row.FindFirst();
    end;

    // An unknown set: the session is not fully composed, so a denial is refused, not raised.
    [Test]
    procedure DenialUnderAnUnknownSetIsRefused()
    var
        Row: Record "PX Row";
    begin
        Lower('PX No Such Set');
        Row.Id := 5;
        Row.Insert();
    end;
}

// Under the test runner's sets (not composed; assumed to grant everything): a set assigned on top of them leaves a
// session whose content is not known (ADR-062 probe P25, stage 4).
codeunit 50994 "PX On Runner"
{
    Subtype = Test;

    [Test]
    procedure AssignOnTopOfTheRunnerSets()
    var
        Row: Record "PX Row";
        Mock: Codeunit "Permissions Mock";
    begin
        Mock.Assign('PX None');
        Row.Id := 9;
        Row.Insert();
    end;
}

codeunit 130302 "Set State Test Run"
{
    SingleInstance = true;
    trigger OnRun()
    var
        PermissionsMock: Codeunit "Permissions Mock";
    begin
        PermissionsMock.Start();
        PermissionsMock.Set('D365 Full Access');
    end;
}
"#;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

/// A usable .NET host that nothing calls (PermissionTestHelper is a registry type).
struct HasDotNet;

impl al_host::dotnet::DotNetHost for HasDotNet {
    fn availability(&self) -> Result<(), al_host::dotnet::DotNetUnavailable> {
        Ok(())
    }
    fn open_session(&self, cfg: &al_host::dotnet::DotNetSessionConfig) -> Box<dyn al_host::dotnet::DotNetSession> {
        al_host::dotnet::NoDotNet.open_session(cfg)
    }
}

fn run_with(opts: &RunOptions) -> CorpusRuntime {
    let manifest = |p: &str| Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        logo: None,
        description: None,
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        translation_files: Vec::new(),
        runtime: None,
        app_json_keys: Vec::new(),
        symbols: vec![],
    };
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], MAIN), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        al_vm::dotnet::DotNetConfig {
            host: Arc::new(HasDotNet),
            ..Default::default()
        },
        &[SuiteRun::everything()],
        opts,
    )
    .expect("the run starts")
}

fn results() -> &'static CorpusRuntime {
    static RT: std::sync::OnceLock<CorpusRuntime> = std::sync::OnceLock::new();
    RT.get_or_init(|| run_with(&RunOptions::default()))
}

fn outcome(rt: &CorpusRuntime, test: &str) -> Outcome {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
        .outcome
        .clone()
}

fn passes(test: &str) {
    assert_eq!(outcome(results(), test), Outcome::Pass, "{test}");
}

fn refused(test: &str, reason: &str) {
    match outcome(results(), test) {
        Outcome::NotRun(why) => assert!(why.contains(reason), "{test}: {why}"),
        other => panic!("{test}: {other:?}"),
    }
}

#[test]
fn a_direct_insert_under_a_read_only_set_raises_the_pinned_error() {
    passes("DirectInsertUnderR");
}

#[test]
fn rename_needs_modify() {
    passes("RenameUnderR");
    passes("RenameUnderRim");
}

#[test]
fn delete_all_and_modify_all_fail_on_a_non_empty_table_and_not_on_an_empty_one() {
    passes("DeleteAllAndModifyAllUnderR");
    passes("DeleteAllOnAnEmptyTableUnderR");
}

#[test]
fn a_trigger_write_uses_the_triggering_tables_permissions_property() {
    passes("TriggerWrites");
}

#[test]
fn an_object_grant_without_the_users_indirect_grant_gives_indirect_delete() {
    passes("IndirectDelete");
}

#[test]
fn record_ref_writes_are_checked_and_temporary_ones_are_not() {
    passes("RecordRefInsert");
    passes("TemporaryWrites");
}

#[test]
fn the_permissions_property_covers_only_the_objects_own_code() {
    passes("FrameRule");
}

#[test]
fn extensions_and_exclusions_compose() {
    passes("ExtensionGrants");
    passes("ExcludingIndirectLeavesIndirect");
}

#[test]
fn a_wildcard_set_grants_its_level_on_every_table() {
    passes("WildcardRead");
}

#[test]
fn the_app_name_is_the_app_of_the_code() {
    passes("AppOfTheCode");
}

#[test]
fn table_inherent_permissions_grant() {
    passes("TableInherentPermissions");
}

#[test]
fn object_and_indirect_inherent_permissions_grant_under_their_assumption() {
    let r = results().results().iter().find(|r| r.test == "ObjectAndIndirectInherentPermissions").expect("ran");
    assert_eq!(r.outcome, Outcome::Pass);
    assert_eq!(r.assumptions, vec![al_builtins::assumptions::Assumption::InherentPermissionsIndirectAndObjects]);
}

#[test]
fn allowed_reads_run_and_a_test_may_end_lowered() {
    passes("AllowedReadsRun");
}

#[test]
fn denials_in_unpinned_shapes_are_refused() {
    refused("DeniedReadIsRefused", "not pinned");
    refused("DenialUnderAnUnknownSetIsRefused", "not fully composed");
    refused("AssignOnTopOfTheRunnerSets", "P25");
}

/// `--permissions off` brings back the refusal of every test that lowers its permissions.
#[test]
fn permissions_off_refuses_as_before() {
    let rt = run_with(&RunOptions {
        permissions_off: true,
        ..Default::default()
    });
    match outcome(&rt, "DirectInsertUnderR") {
        Outcome::NotRun(why) => assert!(why.contains("AddEffectivePermissionSet"), "{why}"),
        other => panic!("{other:?}"),
    }
}
