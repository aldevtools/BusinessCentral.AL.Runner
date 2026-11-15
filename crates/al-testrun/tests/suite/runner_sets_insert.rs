//! The Insert the test runner's permission sets deny (al-runner issue 8160, `al-vm`
//! `permissions/permission_probed/runner_sets_insert.rs`): the measured cell raises BC's error
//! (al-language-testlibraries/test-framework/testpermissions/BiTpDefault.al:35-45), and its neighbors keep main's answer
//! (a pass under assumption `runner-permission-sets-grant-all`).

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

permissionset 131006 "All Objects"
{
    Assignable = true;
    Permissions = table * = X, report * = X, codeunit * = X, page * = X, xmlport * = X, query * = X;
}

table 50960 "RS Other"
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

permissionset 50961 "D365 Full Access"
{
    Assignable = true;
    Permissions = tabledata "RS Other" = RIMD;
}

permissionset 50962 "Test Tables"
{
    Assignable = true;
    Permissions = tabledata "RS Other" = R;
}

permissionset 50963 "D365 Basic"
{
    Assignable = true;
    Permissions = tabledata "RS Other" = R;
}

permissionset 50964 "LOCAL"
{
    Assignable = true;
    Permissions = tabledata "RS Other" = R;
}

// The calls of Library - Lower Permissions.StartLoggingNAVPermissions('D365 Full Access') (BiTpHelper.al:6-10).
codeunit 130302 "Set State Test Run"
{
    SingleInstance = true;
    trigger OnRun()
    var
        PermissionsMock: Codeunit "Permissions Mock";
    begin
        PermissionsMock.Start();
        PermissionsMock.Set('D365 Full Access');
        PermissionsMock.Assign('Test Tables');
        PermissionsMock.Assign('D365 Basic');
        PermissionsMock.Assign('LOCAL');
    end;
}
"#;

const MAIN: &str = r#"
table 50970 "RS Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
    }
    keys
    {
        key(PK; "Code") { Clustered = true; }
    }
}

codeunit 50971 "RS Writer"
{
    procedure InsertRow(RowCode: Code[20])
    var
        Row: Record "RS Row";
    begin
        Row.Code := RowCode;
        Row.Insert();
    end;

    procedure InsertRowTrue(RowCode: Code[20])
    var
        Row: Record "RS Row";
    begin
        Row.Code := RowCode;
        Row.Insert(true);
    end;
}

codeunit 50972 "RS Default"
{
    Subtype = Test;

    [Test]
    procedure InsertThroughHelper()
    var
        Writer: Codeunit "RS Writer";
    begin
        Writer.InsertRow('A');
    end;

    [Test]
    procedure InsertInTheTestMethod()
    var
        Row: Record "RS Row";
    begin
        Row.Code := 'B';
        Row.Insert();
    end;

    [Test]
    procedure InsertTrueThroughHelper()
    var
        Writer: Codeunit "RS Writer";
    begin
        Writer.InsertRowTrue('C');
    end;
}

codeunit 50973 "RS Seeded"
{
    Subtype = Test;

    [Test]
    [TestPermissions(TestPermissions::Disabled)]
    procedure Seed()
    var
        Writer: Codeunit "RS Writer";
    begin
        Writer.InsertRow('S');
    end;

    [Test]
    procedure InsertIntoATableWithRows()
    var
        Writer: Codeunit "RS Writer";
    begin
        Writer.InsertRow('D');
    end;
}

codeunit 50974 "RS Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure InsertWithoutTheRunnersSets()
    var
        Writer: Codeunit "RS Writer";
    begin
        Writer.InsertRow('E');
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

fn run(base: &str) -> CorpusRuntime {
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
        package_data: false,
        package_data_files: Vec::new(),
        translation_files: Vec::new(),
        translation_source: None,
        runtime: None,
        app_json_keys: Vec::new(),
        platform_internals_visible_to: Vec::new(),
        symbols: vec![],
    };
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], MAIN), app("base", &[], base)],
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

/// Main's answer under the runner's sets: a pass flagged by `runner-permission-sets-grant-all`.
fn main_pass(rt: &CorpusRuntime, test: &str) {
    let r = result(rt, test);
    assert_eq!(r.outcome, Outcome::Pass, "{test}");
    assert_eq!(r.assumptions, vec![Assumption::RunnerPermissionSetsGrantAll], "{test}");
}

/// The measured cell: a procedure of a codeunit of the test's app inserts into an empty table of that app with `Insert()`.
#[test]
fn the_measured_insert_under_the_runners_sets_raises_the_denial() {
    let rt = run(BASE);
    assert_eq!(
        result(&rt, "InsertThroughHelper").outcome,
        Outcome::Fail(
            "Sorry, the current permissions prevented the action. (TableData 50970 RS Row Insert: main)".into()
        )
    );
}

/// Neighbors of the measured cell keep main's answer: the insert in the test method itself, `Insert(true)`, and a table
/// that holds a row.
#[test]
fn neighbors_of_the_measured_insert_keep_mains_answer() {
    let rt = run(BASE);
    main_pass(&rt, "InsertInTheTestMethod");
    main_pass(&rt, "InsertTrueThroughHelper");
    main_pass(&rt, "InsertIntoATableWithRows");
    let r = result(&rt, "InsertWithoutTheRunnersSets");
    assert_eq!(r.outcome, Outcome::Pass);
    assert!(r.assumptions.is_empty(), "{:?}", r.assumptions);
}

/// A runner step that assigns another list of sets than the measured one keeps main's answer.
#[test]
fn another_list_of_runner_sets_keeps_mains_answer() {
    let rt = run(&BASE.replace("        PermissionsMock.Assign('LOCAL');\n", ""));
    main_pass(&rt, "InsertThroughHelper");
}
