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
    let manifest = |p: &str| Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
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
        &RunOptions::default(),
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
