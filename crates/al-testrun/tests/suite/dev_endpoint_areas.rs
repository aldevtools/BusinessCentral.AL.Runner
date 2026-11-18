//! `--dev-endpoint-ui` and the session's application areas (al-runner issue 8333). Microsoft's Test Runner app empties
//! them before every test method (ALTestRunnerResetEnvironment.Codeunit.al:27; corpus permissions/testpermissions/BaTpEnv.al:53-55
//! reads the empty text at the start of a test), which the default mode models. The dev-endpoint test runner skips
//! that step, and what the areas hold there is not measured: under the flag, a read of the areas in a test that has
//! not set them is refused. Off, every test keeps main's result.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun, TestResult};

const SRC: &str = r#"
page 50972 "Dev Endpoint Areas Card"
{
    PageType = Card;
    layout
    {
        area(Content)
        {
            field(Info; InfoText) { ApplicationArea = All; }
        }
    }
    var
        InfoText: Text;
}

codeunit 50971 "Dev Endpoint Areas"
{
    Subtype = Test;

    [Test]
    procedure ReadAreas_NotSet()
    begin
        if ApplicationArea() <> '' then
            Error('areas %1', ApplicationArea());
    end;

    [Test]
    procedure OpenTestPage_NotSet()
    var
        Card: TestPage "Dev Endpoint Areas Card";
    begin
        Card.OpenView();
        Card.Close();
    end;

    [Test]
    procedure SetsAreas()
    begin
        ApplicationArea('#Basic');
    end;

    [Test]
    procedure ReadAreas_SetInEarlierTest()
    begin
        if ApplicationArea() <> '' then
            Error('areas %1', ApplicationArea());
    end;

    [Test]
    procedure ReadAreas_AfterSet()
    begin
        ApplicationArea('#Basic');
        if ApplicationArea() <> '#Basic' then
            Error('areas %1', ApplicationArea());
    end;

    [Test]
    procedure ReadAreas_AfterSetEmpty()
    begin
        ApplicationArea('');
        if ApplicationArea() <> '' then
            Error('areas %1', ApplicationArea());
    end;

    [Test]
    procedure OpenTestPage_AfterSet()
    var
        Card: TestPage "Dev Endpoint Areas Card";
    begin
        ApplicationArea('#Basic');
        Card.OpenView();
        Card.Close();
    end;

    [Test]
    procedure NoRead()
    begin
    end;
}
"#;

fn run(dev_endpoint_ui: bool) -> CorpusRuntime {
    CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            dev_endpoint_ui,
            ..Default::default()
        },
    )
}

fn result<'a>(rt: &'a CorpusRuntime, test: &str) -> &'a TestResult {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
}

const ALL: [&str; 8] = [
    "ReadAreas_NotSet",
    "OpenTestPage_NotSet",
    "SetsAreas",
    "ReadAreas_SetInEarlierTest",
    "ReadAreas_AfterSet",
    "ReadAreas_AfterSetEmpty",
    "OpenTestPage_AfterSet",
    "NoRead",
];

/// Off, the runner models the Test Runner app: every test starts with the empty text and passes, a test after one that
/// set the areas included (BaTpEnv.al:53-55; session-system/TestSessionApplicationAreaTestBoundary.al:56-59).
#[test]
fn off_every_test_starts_with_no_areas() {
    let rt = run(false);
    for test in ALL {
        assert_eq!(result(&rt, test).outcome, Outcome::Pass, "{test}");
    }
}

/// On, a read of the areas (`ApplicationArea()`, a page open) in a test that has not set them is refused, also when an
/// earlier test set them.
#[test]
fn on_a_read_before_the_test_sets_the_areas_is_refused() {
    let rt = run(true);
    for test in ["ReadAreas_NotSet", "OpenTestPage_NotSet", "ReadAreas_SetInEarlierTest"] {
        let r = result(&rt, test);
        assert!(
            matches!(&r.outcome, Outcome::NotRun(m) if m.contains("--dev-endpoint-ui")),
            "{test}: {:?}",
            r.outcome
        );
    }
}

/// On, a test that sets the areas before it reads them, or reads none, keeps the result it has with the flag off.
#[test]
fn on_a_test_that_sets_the_areas_first_keeps_its_result() {
    let (off, on) = (run(false), run(true));
    for test in [
        "SetsAreas",
        "ReadAreas_AfterSet",
        "ReadAreas_AfterSetEmpty",
        "OpenTestPage_AfterSet",
        "NoRead",
    ] {
        assert_eq!(result(&on, test).outcome, result(&off, test).outcome, "{test}");
    }
}
