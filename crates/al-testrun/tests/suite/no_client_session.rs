//! `--no-client` (al-runner issue 7564, `RunOptions::no_client`): the test session is a session without a client.
//! Pins: corpus pages/background-tasks/BjNoGuiDialogs.al:8-10 (GuiAllowed false, Message returns and reaches no
//! MessageHandler, Confirm raises CallbackNotAllowed whatever its default), measured in a page background task worker.
//! StrMenu raises it too (BjNoClientDialogs.al `01 StrMenu with instruction`, corpus pull request 1286; the other dialogs of
//! that probe are tested in al-vm `tests/host/no_client_session.rs`). That a web-service session answers the same is the
//! assumption `no-client-session`: each such pass is flagged. A call the probes do not cover is not run. Off, nothing changes.

use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun, TestResult};

// Codeunit 40 stands in for Base Application's LogInManagement (as in `the_test_session_starts_on_the_login_work_date`,
// test-framework/RlWorkDateAtStart.al:47-52): its login code runs only in a session with a client.
const SRC: &str = r#"
codeunit 40 LogInManagement
{
    procedure GetDefaultWorkDate(): Date begin exit(20240115D); end;
}

page 50971 "No Client Card"
{
    PageType = Card;
}

codeunit 50970 "No Client"
{
    Subtype = Test;

    [Test]
    procedure OpensTestPage()
    var
        Card: TestPage "No Client Card";
    begin
        Card.OpenView();
        Card.Close();
    end;

    [Test]
    procedure Gui()
    begin
        if GuiAllowed() then Error('GuiAllowed is true');
    end;

    [Test]
    procedure Say()
    begin
        Message('Done %1', 7);
    end;

    [Test]
    procedure Ask()
    begin
        asserterror Confirm('Go on?', true);
        if GetLastErrorCode() <> 'CallbackNotAllowed' then Error('code %1', GetLastErrorCode());
    end;

    [Test]
    procedure Menu()
    begin
        asserterror StrMenu('Alpha,Beta', 1, 'Pick');
        if GetLastErrorCode() <> 'CallbackNotAllowed' then Error('code %1', GetLastErrorCode());
        if GetLastErrorText() <> 'Microsoft Dynamics 365 Business Central Server attempted to issue a client callback to show a selection menu: Pick (CodeUnit 50970 No Client). Client callbacks are not supported on Microsoft Dynamics 365 Business Central Server.' then
            Error(GetLastErrorText());
    end;

    [Test]
    procedure ClientType()
    begin
        if Format(CurrentClientType()) = '' then;
    end;

    [Test]
    procedure ReadsWorkDate()
    begin
        if WorkDate() = 0D then Error('no work date');
    end;

    [Test]
    procedure Nothing()
    begin
    end;
}
"#;

fn run(no_client: bool) -> CorpusRuntime {
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
            no_client,
            ..Default::default()
        },
    )
}

fn result<'a>(rt: &'a CorpusRuntime, test: &str) -> &'a TestResult {
    rt.results()
        .iter()
        .find(|t| t.test == test)
        .unwrap_or_else(|| panic!("no result for {test}"))
}

fn flagged(rt: &CorpusRuntime, test: &str) -> bool {
    result(rt, test).assumptions.contains(&Assumption::NoClientSession)
}

#[test]
fn gui_allowed_message_and_confirm_answer_as_without_a_client_and_are_flagged() {
    let rt = run(true);
    for test in ["Gui", "Say", "Ask", "Menu"] {
        assert_eq!(result(&rt, test).outcome, Outcome::Pass, "{test}");
        assert!(flagged(&rt, test), "{test} is not flagged");
    }
}

#[test]
fn a_call_the_probes_do_not_cover_is_not_run() {
    let rt = run(true);
    match &result(&rt, "ClientType").outcome {
        Outcome::NotRun(why) => assert!(why.contains("CurrentClientType in a session without a client"), "{why}"),
        other => panic!("{other:?}"),
    }
}

// The login code that sets WorkDate runs only in a session with a client (Base Application `LogInManagement.CompanyOpen`
// asks GuiAllowed). That GuiAllowed is false in the session is the assumption, so a test that reads the WorkDate the
// login code would have set is flagged; a test that does not read it is not.
#[test]
fn a_work_date_read_after_the_skipped_login_code_is_flagged() {
    let rt = run(true);
    assert_eq!(result(&rt, "ReadsWorkDate").outcome, Outcome::Pass);
    assert!(flagged(&rt, "ReadsWorkDate"));
    assert_eq!(result(&rt, "Nothing").outcome, Outcome::Pass);
    assert!(!flagged(&rt, "Nothing"));
}

// Whether a session without a client can open a TestPage is not pinned: the test runs as in a session with a client
// and is flagged.
#[test]
fn a_test_page_opens_and_the_pass_is_flagged() {
    let rt = run(true);
    assert_eq!(result(&rt, "OpensTestPage").outcome, Outcome::Pass);
    assert!(flagged(&rt, "OpensTestPage"));
}

#[test]
fn off_nothing_is_flagged_and_the_dialogs_end_as_before() {
    let rt = run(false);
    for test in [
        "Gui",
        "Say",
        "Ask",
        "Menu",
        "ClientType",
        "ReadsWorkDate",
        "OpensTestPage",
        "Nothing",
    ] {
        assert!(!flagged(&rt, test), "{test}");
    }
    // GuiAllowed is true in the test session (BjNoGuiDialogs.al:8); the three dialogs have no handler (Unhandled UI).
    for test in ["Gui", "Say", "Ask", "Menu"] {
        assert!(
            matches!(result(&rt, test).outcome, Outcome::Fail(_)),
            "{test}: {:?}",
            result(&rt, test).outcome
        );
    }
    for test in ["ClientType", "ReadsWorkDate", "OpensTestPage"] {
        assert_eq!(result(&rt, test).outcome, Outcome::Pass, "{test}");
    }
}
