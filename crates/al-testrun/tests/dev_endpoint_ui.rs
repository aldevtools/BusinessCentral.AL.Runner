//! `--dev-endpoint-ui` (al-runner issue 3697, `RunOptions::dev_endpoint_ui`): a UI call no handler takes gets the
//! answer Business Central's dev-endpoint test runner gives it, for the shapes measured there, and the pass is flagged
//! `dev-endpoint-ui` with the answered calls on the result. Off, and for a shape not measured, the call fails with
//! "Unhandled UI" as under Microsoft's Test Runner app (handlers/TestUnhandledUiAndDialogDefaults.al:68-107).

use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun, TestResult};

const SRC: &str = r#"
codeunit 50960 "Dev Endpoint UI"
{
    Subtype = Test;

    [Test]
    procedure SessionUpdate_NoHandler()
    var
        Settings: SessionSettings;
    begin
        Settings.Init();
        Settings.RequestSessionUpdate(false);
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

/// A shape the dev-endpoint mode was not measured for keeps the outcome it has without the flag, and is not marked.
#[test]
fn a_shape_not_measured_ends_as_it_does_without_the_flag() {
    let (off, on) = (run(false), run(true));
    let test = "SessionUpdate_NoHandler";
    assert_eq!(result(&on, test).outcome, result(&off, test).outcome);
    assert!(!matches!(result(&on, test).outcome, Outcome::Pass), "{:?}", result(&on, test).outcome);
    assert!(result(&on, test).ui_answered.is_empty());
    assert!(!result(&on, test).assumptions.contains(&Assumption::DevEndpointUi));
}
