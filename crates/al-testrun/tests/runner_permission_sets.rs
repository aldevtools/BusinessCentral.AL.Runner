//! The permission sets BC's test runner assigns before each test method (Tests-TestLibraries 27.5: codeunit 130302 "Set
//! State Test Run" -> Permission Test Catalog.InitializePermissionSetForTest -> Library - Lower Permissions
//! .StartLoggingNAVPermissions('D365 Full Access') -> Permissions Mock.Set and Assign -> DotNet
//! PermissionTestHelper.AddEffectivePermissionSet). Permissions are not enforced here (ADR-027, owner decision 2): the
//! runner's step runs, and every test after it is a flagged pass (assumption `runner-permission-sets-grant-all`). A
//! test that assigns a permission set itself is still refused.

use al_builtins::assumptions::Assumption;
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
    trigger OnRun()
    begin
        if Started then
            Stop()
        else
            Start();
    end;
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
}

// Own declarations with the grants ADR-062 derives for System Application's sets: SUPER grants everything.
permissionset 131006 "All Objects"
{
    Assignable = true;
    Permissions = table * = X, report * = X, codeunit * = X, page * = X, xmlport * = X, query * = X;
}

permissionset 50990 "PX Data"
{
    Assignable = true;
    Permissions = tabledata * = RIMD;
}

permissionset 50991 "PX System"
{
    Assignable = false;
    Permissions = system * = X;
}

permissionset 31 SUPER
{
    Assignable = true;
    IncludedPermissionSets = "All Objects", "PX Data", "PX System";
}

permissionset 50992 "PX Read"
{
    Assignable = true;
    Permissions = tabledata "PX Item" = R;
}

table 50990 "PX Item"
{
    fields
    {
        field(1; "No."; Code[20]) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 130302 "Set State Test Run"
{
    SingleInstance = true;
    trigger OnRun()
    var
        PermissionsMock: Codeunit "Permissions Mock";
    begin
        PermissionsMock.Set('D365 Full Access');
        PermissionsMock.Assign('D365 BASIC');
    end;
}
"#;

const MAIN: &str = r#"
codeunit 50930 "Perm Sets"
{
    Subtype = Test;

    [Test]
    procedure RunsUnderTheRunnersSets()
    begin
    end;

    [Test]
    procedure LowersItsOwnPermissions()
    var
        PermissionsMock: Codeunit "Permissions Mock";
    begin
        PermissionsMock.Assign('D365 READ');
    end;
}

codeunit 50932 "Perm Sets Super"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure SetSuperThenWrite()
    var
        PXItem: Record "PX Item";
        PermissionsMock: Codeunit "Permissions Mock";
    begin
        PermissionsMock.Start();
        PermissionsMock.Set('SUPER');
        PXItem."No." := 'A';
        PXItem.Insert();
        if not PXItem.Get('A') then
            Error('row not found');
        PermissionsMock.Stop();
    end;

    [Test]
    procedure AssignSuperThenWrite()
    var
        PXItem: Record "PX Item";
        PermissionsMock: Codeunit "Permissions Mock";
    begin
        PermissionsMock.Start();
        PermissionsMock.Assign('SUPER');
        PXItem."No." := 'B';
        PXItem.Insert();
        PermissionsMock.Stop();
    end;

    [Test]
    procedure SetDataWithoutSystemThenWrite()
    var
        PXItem: Record "PX Item";
        PermissionsMock: Codeunit "Permissions Mock";
    begin
        PermissionsMock.Start();
        PermissionsMock.Set('PX Data');
        PXItem."No." := 'C';
        PXItem.Insert();
    end;

    [Test]
    procedure SetSuperThenNarrowThenWrite()
    var
        PXItem: Record "PX Item";
        PermissionsMock: Codeunit "Permissions Mock";
    begin
        PermissionsMock.Start();
        PermissionsMock.Set('SUPER');
        PermissionsMock.Set('PX Read');
        PXItem."No." := 'D';
        PXItem.Insert();
    end;

    [Test]
    procedure NarrowThenSuperThenWrite()
    var
        PXItem: Record "PX Item";
        PermissionsMock: Codeunit "Permissions Mock";
    begin
        PermissionsMock.Start();
        PermissionsMock.Set('PX Read');
        PermissionsMock.Assign('SUPER');
        PXItem."No." := 'E';
        PXItem.Insert();
        PermissionsMock.Stop();
    end;
}

codeunit 50933 "Perm Sets Super On Runner"
{
    Subtype = Test;

    [Test]
    procedure AddSuperToTheRunnersSets()
    var
        PXItem: Record "PX Item";
        PermissionsMock: Codeunit "Permissions Mock";
    begin
        PermissionsMock.Assign('SUPER');
        PXItem."No." := 'F';
        PXItem.Insert();
    end;
}

codeunit 50931 "Perm Sets Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure RunsWithoutTheRunnersSets()
    begin
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

fn run() -> CorpusRuntime {
    run_with(&RunOptions::default())
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

fn result<'a>(rt: &'a CorpusRuntime, test: &str) -> &'a al_testrun::runtime::TestResult {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
}

#[test]
fn the_runners_permission_sets_give_a_flagged_pass() {
    let rt = run();
    let r = result(&rt, "RunsUnderTheRunnersSets");
    assert_eq!(r.outcome, Outcome::Pass);
    assert_eq!(r.assumptions, vec![Assumption::RunnerPermissionSetsGrantAll]);
}

#[test]
fn a_test_that_assigns_a_permission_set_itself_is_still_refused() {
    let rt = run();
    match &result(&rt, "LowersItsOwnPermissions").outcome {
        Outcome::NotRun(why) => assert!(why.contains("AddEffectivePermissionSet"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_test_without_test_permissions_gets_no_runner_sets_and_a_clean_pass() {
    let rt = run();
    let r = result(&rt, "RunsWithoutTheRunnersSets");
    assert_eq!(r.outcome, Outcome::Pass);
    assert!(r.assumptions.is_empty(), "{:?}", r.assumptions);
}

/// `--ignore-permission-mock` (al-runner issue 5996): a test that lowers its own permissions runs with full permissions.
/// Its pass is flagged `permissions-not-enforced`, so it is never read as a permission check.
#[test]
fn with_ignore_permission_mock_a_test_that_lowers_its_permissions_runs_and_is_flagged() {
    let rt = run_with(&RunOptions {
        ignore_permission_mock: true,
        ..Default::default()
    });
    let r = result(&rt, "LowersItsOwnPermissions");
    assert_eq!(r.outcome, Outcome::Pass);
    assert!(
        r.assumptions.contains(&Assumption::PermissionsNotEnforced),
        "{:?}",
        r.assumptions
    );
    // The other tests keep their outcomes and flags.
    let r = result(&rt, "RunsUnderTheRunnersSets");
    assert_eq!(r.assumptions, vec![Assumption::RunnerPermissionSetsGrantAll]);
    assert!(result(&rt, "RunsWithoutTheRunnersSets").assumptions.is_empty());
}

fn not_run_lowered(rt: &CorpusRuntime, test: &str) {
    match &result(rt, test).outcome {
        Outcome::NotRun(why) => assert!(
            why.contains("AddEffectivePermissionSet") && why.contains("lowers"),
            "{why}"
        ),
        other => panic!("{test}: {other:?}"),
    }
}

/// A pass flagged only by the assumption ADR-062 probe P0 confirms: sets that grant everything leave the session
/// unrestricted.
fn unrestricted_pass(rt: &CorpusRuntime, test: &str) {
    let r = result(rt, test);
    assert_eq!(r.outcome, Outcome::Pass, "{test}");
    assert_eq!(
        r.assumptions,
        vec![Assumption::PermissionSetsGrantingEverythingUnrestricted],
        "{test}"
    );
}

/// al-runner issue 6296: `Permissions Mock.Set('SUPER')` (Clear, Assign('All Objects'), Assign('SUPER')) and
/// `Assign('SUPER')` leave a session whose sets grant everything (ADR-062 section 1: the union of the assigned sets,
/// TestPermissionsMockEnforcement.al:28-72; SUPER's content from its source). The test runs, flagged only by
/// `permission-sets-granting-everything-unrestricted` until probe P0 lands, never by `permissions-not-enforced`.
#[test]
fn a_session_lowered_to_super_is_unrestricted_and_passes_clean() {
    let rt = run();
    unrestricted_pass(&rt, "SetSuperThenWrite");
    unrestricted_pass(&rt, "AssignSuperThenWrite");
    // Lowered first: the test's own call to add SUPER is already an access of a restricted session, refused while no
    // access kind is enforced (on BC All Objects grants its Execute).
    not_run_lowered(&rt, "NarrowThenSuperThenWrite");
}

/// A list that does not grant everything stays refused: All Objects with all table data lacks Execute on system
/// objects, and a later `Set` of a narrow set lowers the session again.
#[test]
fn a_list_that_does_not_grant_everything_is_still_refused() {
    let rt = run();
    not_run_lowered(&rt, "SetDataWithoutSystemThenWrite");
    not_run_lowered(&rt, "SetSuperThenNarrowThenWrite");
}

/// SUPER assigned on top of the runner's sets (`Library - Lower Permissions.AddPermissionSet('SUPER')`): the runner's
/// sets keep their assumption, and the test's own SUPER adds the unrestricted one.
#[test]
fn super_added_to_the_runners_sets_passes_under_the_two_permission_assumptions() {
    let rt = run();
    let r = result(&rt, "AddSuperToTheRunnersSets");
    assert_eq!(r.outcome, Outcome::Pass);
    let mut got = r.assumptions.clone();
    got.sort();
    let mut want = vec![
        Assumption::RunnerPermissionSetsGrantAll,
        Assumption::PermissionSetsGrantingEverythingUnrestricted,
    ];
    want.sort();
    assert_eq!(got, want);
}
