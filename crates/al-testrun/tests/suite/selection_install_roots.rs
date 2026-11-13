//! al-runner issue 6924: a selection (`--explain`, `--codeunit`, `--test`) runs under the same installed state as the full
//! run. The install code of another app may run a test codeunit by an id no text of the workspace shows, also through a
//! dependency app's code; the selection left that codeunit out of the compile ("not compiled: no code reached it while
//! compiling the workspace's dependencies"). The run sees it and runs the selection again on the full run's compile
//! (al-testrun `runtime/narrowing.rs`).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions};
use al_testrun::{CaseId, Outcome};

const DEP_ID: &str = "11111111-1111-1111-1111-111111169240";
const TEST_ID: &str = "22222222-2222-2222-2222-222222269240";

const DEP: &str = r#"
table 50400 "SI Setting"
{
    fields { field(1; Code; Code[20]) { } field(2; Value; Text[30]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

codeunit 50400 "SI Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        S: Record "SI Setting";
    begin
        S.Code := 'SEEDED';
        S.Value := 'yes';
        S.Insert();
    end;
}
"#;

const TESTS: &str = r#"
codeunit 50410 "SI Tests A"
{
    Subtype = Test;

    [Test]
    procedure ReadsInstalledRow()
    var
        S: Record "SI Setting";
    begin
        S.Get('SEEDED');
        if S.Value <> 'yes' then
            Error('value %1', S.Value);
    end;
}

codeunit 50411 "SI Tests B"
{
    Subtype = Test;

    [Test]
    procedure AlsoReadsInstalledRow()
    var
        S: Record "SI Setting";
    begin
        if not S.Get('SEEDED') then
            Error('not installed');
    end;
}
"#;

fn run(only: Option<Vec<(usize, u32)>>, test: &str, codeunit: u32) -> Outcome {
    let apps = vec![
        AppSource {
            id: TEST_ID.into(),
            name: "SI Tests".into(),
            dependencies: vec![DEP_ID.into()],
            files: vec![("t.al".into(), TESTS.into())],
        },
        AppSource {
            id: DEP_ID.into(),
            name: "SI Dep".into(),
            dependencies: Vec::new(),
            files: vec![("d.al".into(), DEP.into())],
        },
    ];
    let kinds = vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency];
    let opts = RunOptions {
        only,
        ..Default::default()
    };
    let rt = CorpusRuntime::run_suites_with(
        apps,
        kinds,
        &[Manifest::default(), Manifest::default()],
        &Default::default(),
        None,
        Default::default(),
        &[al_testrun::runtime::SuiteRun::everything()],
        &opts,
    );
    rt.first_outcome(&CaseId {
        codeunit,
        name: test.into(),
    })
}

#[test]
fn the_full_run_reads_the_install_row() {
    assert_eq!(run(None, "ReadsInstalledRow", 50410), Outcome::Pass);
}

/// The selection must give the same outcome as the full run.
#[test]
fn a_selected_codeunit_reads_the_install_row_of_a_dependency_app() {
    assert_eq!(run(Some(vec![(0, 50410)]), "ReadsInstalledRow", 50410), Outcome::Pass);
    assert_eq!(
        run(Some(vec![(0, 50411)]), "AlsoReadsInstalledRow", 50411),
        Outcome::Pass
    );
}

const DYNAMIC_INSTALL: &str = r#"
codeunit 50421 "SI Dynamic Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Id: Integer;
    begin
        // The id is computed: no name or literal in the text says which codeunit runs.
        Id := 50420 + 3;
        Codeunit.Run(Id);
    end;
}

codeunit 50423 "SI Seeding Tests"
{
    Subtype = Test;

    trigger OnRun()
    var
        S: Record "SI Setting";
    begin
        S.Code := 'SEEDED';
        S.Value := 'yes';
        S.Insert();
    end;

    [Test]
    procedure Mine()
    begin
    end;
}
"#;

/// The install code of a second workspace app runs a test codeunit of that app by a computed id. A selection in the
/// first app compiles that codeunit, as the full run does: the outcome must not depend on which tests are selected
/// (al-runner issue 6924).
#[test]
fn a_selection_compiles_the_test_codeunit_an_install_routine_runs_by_a_computed_id() {
    let outcome = |only: Option<Vec<(usize, u32)>>| {
        let apps = vec![
            AppSource {
                id: TEST_ID.into(),
                name: "SI Tests".into(),
                dependencies: vec!["33333333-3333-3333-3333-333333369240".into()],
                files: vec![(
                    "t.al".into(),
                    format!("{}{TESTS}", &DEP[..DEP.find("codeunit 50400").unwrap()]),
                )],
            },
            AppSource {
                id: DEP_ID.into(),
                name: "SI Dynamic".into(),
                dependencies: vec![TEST_ID.into()],
                files: vec![("d.al".into(), DYNAMIC_INSTALL.to_string())],
            },
            // A dependency app makes the compile demand-driven, as Microsoft's apps do.
            AppSource {
                id: "33333333-3333-3333-3333-333333369240".into(),
                name: "SI Lib".into(),
                dependencies: Vec::new(),
                files: vec![(
                    "l.al".into(),
                    "codeunit 50300 \"SI Lib\" { procedure A(): Integer begin exit(1); end; }".into(),
                )],
            },
        ];
        let kinds = vec![
            al_sema::AppKind::Workspace,
            al_sema::AppKind::Workspace,
            al_sema::AppKind::Dependency,
        ];
        let opts = RunOptions {
            only,
            ..Default::default()
        };
        let rt = CorpusRuntime::run_suites_with(
            apps,
            kinds,
            &vec![Manifest::default(); 3],
            &Default::default(),
            None,
            Default::default(),
            &[al_testrun::runtime::SuiteRun::everything()],
            &opts,
        );
        rt.first_outcome(&CaseId {
            codeunit: 50410,
            name: "ReadsInstalledRow".into(),
        })
    };
    assert_eq!(outcome(None), Outcome::Pass, "full run");
    assert_eq!(outcome(Some(vec![(0, 50410)])), Outcome::Pass, "selected codeunit");
}

const RUNNER_LIB: &str = r#"
codeunit 50300 "SI Runner"
{
    procedure RunById(Id: Integer)
    begin
        Codeunit.Run(Id);
    end;
}
"#;

const TEST_MANAGEMENT: &str = r#"
codeunit 50430 "SI Test Mgt Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Runner: Codeunit "SI Runner";
    begin
        // The id reaches a dependency app's code, which runs the codeunit: nothing in the workspace's text runs it.
        Runner.RunById(50431);
    end;
}

codeunit 50431 "SI Replacement Tests"
{
    Subtype = Test;

    trigger OnRun()
    var
        S: Record "SI Setting";
    begin
        S.Code := 'SEEDED';
        S.Value := 'yes';
        S.Insert();
    end;

    [Test]
    procedure Replacement()
    begin
    end;
}
"#;

/// The apps of the test-management shape: the tests (`tests`, with the `SI Setting` table), the test-management app
/// (`management`: its install code and its replacement tests) and the dependency app that runs a codeunit by id.
fn management_apps(tests: &str, management: &str) -> (Vec<AppSource>, Vec<al_sema::AppKind>, Vec<Manifest>) {
    let lib = "33333333-3333-3333-3333-333333369241";
    let apps = vec![
        AppSource {
            id: TEST_ID.into(),
            name: "SI Tests".into(),
            dependencies: Vec::new(),
            files: vec![(
                "t.al".into(),
                format!("{}{tests}", &DEP[..DEP.find("codeunit 50400").unwrap()]),
            )],
        },
        AppSource {
            id: DEP_ID.into(),
            name: "SI Test Mgt".into(),
            dependencies: vec![TEST_ID.into(), lib.into()],
            files: vec![("m.al".into(), management.to_string())],
        },
        AppSource {
            id: lib.into(),
            name: "SI Runner Lib".into(),
            dependencies: Vec::new(),
            files: vec![("l.al".into(), RUNNER_LIB.to_string())],
        },
    ];
    let kinds = vec![
        al_sema::AppKind::Workspace,
        al_sema::AppKind::Workspace,
        al_sema::AppKind::Dependency,
    ];
    (apps, kinds, vec![Manifest::default(); 3])
}

/// The outcome of `case` and whether the selection ran again on the full run's compile, in a one-shot run.
fn management_run(
    tests: &str,
    management: &str,
    only: Option<Vec<(usize, u32)>>,
    case: (u32, &str),
) -> (Outcome, bool) {
    let (apps, kinds, manifests) = management_apps(tests, management);
    let opts = RunOptions {
        only,
        ..Default::default()
    };
    let rt = CorpusRuntime::run_suites_with(
        apps,
        kinds,
        &manifests,
        &Default::default(),
        None,
        Default::default(),
        &[al_testrun::runtime::SuiteRun::everything()],
        &opts,
    );
    let outcome = rt.first_outcome(&CaseId {
        codeunit: case.0,
        name: case.1.into(),
    });
    (outcome, rt.phases.full_redone)
}

/// The install code of a test-management app hands an id to a dependency app, whose code runs the test codeunit of that
/// id (as Microsoft's Test Runner app runs test codeunits by the ids in its tables). The full run compiles every test
/// codeunit; a selection of another app's test must give the same outcome, so it may not run the install code over a
/// compile that left that test codeunit out (al-runner issue 6924, `--explain` on edge 8c658cf4f).
#[test]
fn a_selection_runs_install_code_that_runs_a_test_codeunit_through_a_dependency_app() {
    let case = (50410, "ReadsInstalledRow");
    assert_eq!(
        management_run(TESTS, TEST_MANAGEMENT, None, case),
        (Outcome::Pass, false),
        "full run"
    );
    // The compile for the selection leaves the replacement tests out; the install code meets them, and the selection runs
    // again on the full run's compile.
    assert_eq!(
        management_run(TESTS, TEST_MANAGEMENT, Some(vec![(0, 50410)]), case),
        (Outcome::Pass, true),
        "selected codeunit"
    );
}

/// The same through a warm server (`al-runner serve`, `Warm`): a request that selects one test gets the full run's
/// outcome too. A warm server compiles every test codeunit whatever a request selects (al-runner issue 7978), so the
/// request runs on the full run's compile from the start and is not redone.
#[test]
fn a_warm_server_request_runs_install_code_that_runs_a_test_codeunit_through_a_dependency_app() {
    // What a client of `al-runner serve` is streamed: one `test` line per test as it finishes (docs/manual/tool-protocol.md
    // section 4b), so a request that is redone may not stream the lines of the run it threw away.
    let streamed: std::sync::Arc<std::sync::Mutex<Vec<(String, Outcome)>>> = Default::default();
    let on_test = {
        let streamed = streamed.clone();
        std::sync::Arc::new(move |e: al_testrun::events::TestEvent| {
            streamed.lock().unwrap().push((e.test, e.outcome));
        })
    };
    let opts = RunOptions {
        only: Some(vec![(0, 50410)]),
        hooks: al_testrun::events::Hooks {
            on_test: Some(on_test),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut warm = al_testrun::runtime::Warm::new();
    let (rt, used) = warm
        .run(
            "management",
            &[],
            || management_apps(TESTS, TEST_MANAGEMENT),
            &Default::default(),
            None,
            Default::default(),
            &[al_testrun::runtime::SuiteRun::everything()],
            &opts,
        )
        .expect("the request runs");
    let outcome = rt.first_outcome(&CaseId {
        codeunit: 50410,
        name: "ReadsInstalledRow".into(),
    });
    assert_eq!((outcome, rt.phases.full_redone), (Outcome::Pass, false));
    assert_eq!(
        *streamed.lock().unwrap(),
        [("ReadsInstalledRow".to_string(), Outcome::Pass)],
        "one line for the one test, from the run that counts"
    );
    // The first request compiled the whole workspace.
    assert_eq!(used, al_testrun::runtime::WarmUse::Compiled);
}

const SEQUENCE_TESTS: &str = r#"
codeunit 50410 "SI Tests A"
{
    Subtype = Test;

    [Test]
    procedure ReadsInstalledSequence()
    begin
        if not NumberSequence.Exists('SINSTALLED') then
            Error('no sequence');
    end;
}
"#;

/// The install code leaves state that is not a table (a NumberSequence), through a test codeunit the selection leaves
/// out. The selected test touches no table, so no refused table tells the runner; the install run itself must.
#[test]
fn a_selection_reruns_when_install_code_met_what_it_left_out_and_the_test_touches_no_table() {
    let management = TEST_MANAGEMENT.replace(
        "        S.Code := 'SEEDED';\n        S.Value := 'yes';\n        S.Insert();",
        "        if not NumberSequence.Exists('SINSTALLED') then\n            NumberSequence.Insert('SINSTALLED', 1, 1);",
    );
    assert!(management.contains("NumberSequence.Insert"));
    let case = (50410, "ReadsInstalledSequence");
    assert_eq!(
        management_run(SEQUENCE_TESTS, &management, None, case),
        (Outcome::Pass, false),
        "full run"
    );
    assert_eq!(
        management_run(SEQUENCE_TESTS, &management, Some(vec![(0, 50410)]), case),
        (Outcome::Pass, true),
        "selected codeunit"
    );
}
