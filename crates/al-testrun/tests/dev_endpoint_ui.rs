//! `--dev-endpoint-ui` (al-runner issue 3697, `RunOptions::dev_endpoint_ui`): a UI call no handler takes gets the
//! answer Business Central's dev-endpoint test runner gives it, for the shapes measured there, and the pass is flagged
//! `dev-endpoint-ui` with the answered calls on the result. Off, and for a shape not measured, the call fails with
//! "Unhandled UI" as under Microsoft's Test Runner app (handlers/TestUnhandledUiAndDialogDefaults.al:68-107).

use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun, TestResult};

const SRC: &str = r#"
codeunit 50961 "Dev Endpoint Log"
{
    SingleInstance = true;
    var
        Seen: Text;
    procedure Add(T: Text)
    begin
        Seen += T;
    end;
    procedure Get(): Text
    begin
        exit(Seen);
    end;
    procedure Clear()
    begin
        Seen := '';
    end;
}

page 50962 "Dev Endpoint Card"
{
    PageType = Card;
    layout
    {
        area(Content)
        {
            field(Info; InfoText) { }
        }
    }
    var
        InfoText: Text;
    trigger OnOpenPage()
    var
        Log: Codeunit "Dev Endpoint Log";
    begin
        Log.Add('OnOpenPage;');
    end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    var
        Log: Codeunit "Dev Endpoint Log";
    begin
        Log.Add('OnQueryClosePage;');
        exit(true);
    end;
    trigger OnClosePage()
    var
        Log: Codeunit "Dev Endpoint Log";
    begin
        Log.Add('OnClosePage;');
    end;
}

page 50963 "Dev Endpoint Init Card"
{
    PageType = Card;
    trigger OnInit()
    begin
    end;
}

codeunit 50960 "Dev Endpoint UI"
{
    Subtype = Test;

    var
        Log: Codeunit "Dev Endpoint Log";

    // The shapes of the corpus probe Y53DevRunnerProbe.al (codeunit 97530), measured through the dev endpoint
    // (TestRunnerHub) on a BC 28.5 container: the call returns at once with the value below.
    [Test]
    procedure Confirm_NoDefault()
    begin
        if Confirm('Go on?') then Error('Confirm(Q) answered Yes');
    end;

    [Test]
    procedure Confirm_DefaultTrue()
    begin
        if not Confirm('Go on?', true) then Error('Confirm(Q, true) answered No');
    end;

    [Test]
    procedure Confirm_DefaultFalse()
    begin
        if Confirm('Go on?', false) then Error('Confirm(Q, false) answered Yes');
    end;

    [Test]
    procedure StrMenu_Default2()
    begin
        if StrMenu('Alpha,Beta,Gamma', 2, 'Pick') <> 2 then Error('StrMenu default 2 did not answer 2');
    end;

    [Test]
    procedure StrMenu_NoDefault()
    begin
        if StrMenu('Alpha,Beta,Gamma') <> 1 then Error('StrMenu without a default did not answer 1');
    end;

    [Test]
    procedure StrMenu_DefaultZero()
    begin
        StrMenu('Alpha,Beta,Gamma', 0);
    end;

    [Test]
    procedure Message_NoHandler()
    begin
        Message('Done %1', 7);
    end;

    [Test]
    procedure Hyperlink_NoHandler()
    begin
        Hyperlink('https://example.invalid/y53');
    end;

    [Test]
    [HandlerFunctions('UnusedConfirm')]
    procedure DeclaredHandler_NotUsed()
    begin
    end;

    [Test]
    [HandlerFunctions('ConfirmYes')]
    procedure Confirm_HandlerStillAnswers()
    begin
        if not Confirm('Handled?', false) then Error('the bound handler did not answer');
    end;

    [ConfirmHandler]
    procedure UnusedConfirm(Question: Text[1024]; var Reply: Boolean)
    begin
    end;

    [ConfirmHandler]
    procedure ConfirmYes(Question: Text[1024]; var Reply: Boolean)
    begin
        Reply := true;
    end;

    [Test]
    procedure PageRunModal_NoHandler()
    begin
        Log.Clear();
        if Page.RunModal(Page::"Dev Endpoint Card") <> Action::None then Error('RunModal did not answer None');
        if Log.Get() <> '' then Error('page triggers ran: %1', Log.Get());
    end;

    [Test]
    procedure PageRun_NoHandler()
    begin
        Log.Clear();
        Page.Run(Page::"Dev Endpoint Card");
        if Log.Get() <> '' then Error('page triggers ran: %1', Log.Get());
    end;

    [Test]
    procedure PageRunModal_OnInitPage()
    begin
        Page.RunModal(Page::"Dev Endpoint Init Card");
    end;

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

fn answered(rt: &CorpusRuntime, test: &str) -> Vec<String> {
    let r = result(rt, test);
    assert_eq!(r.outcome, Outcome::Pass, "{test}");
    assert!(r.assumptions.contains(&Assumption::DevEndpointUi), "{test}: {:?}", r.assumptions);
    r.ui_answered.clone()
}

/// Off, every shape fails with "Unhandled UI", as under Microsoft's Test Runner app.
#[test]
fn off_a_ui_call_without_a_handler_fails() {
    let rt = run(false);
    for test in ["Confirm_NoDefault", "StrMenu_Default2", "Message_NoHandler", "Hyperlink_NoHandler"] {
        let r = result(&rt, test);
        assert!(matches!(&r.outcome, Outcome::Fail(m) if m.starts_with("Unhandled UI")), "{test}: {:?}", r.outcome);
        assert!(r.ui_answered.is_empty());
    }
}

/// Confirm answers its default; an omitted default is No (Y53DevRunnerProbe.al Confirm_*_NoHandler, BC 28.5 dev endpoint).
#[test]
fn confirm_answers_its_default() {
    let rt = run(true);
    assert_eq!(answered(&rt, "Confirm_NoDefault"), vec!["Confirm Go on?: false"]);
    assert_eq!(answered(&rt, "Confirm_DefaultTrue"), vec!["Confirm Go on?: true"]);
    assert_eq!(answered(&rt, "Confirm_DefaultFalse"), vec!["Confirm Go on?: false"]);
}

/// StrMenu answers its default, 1 when omitted (Y53DevRunnerProbe.al StrMenu_*_NoHandler). Default 0 was not measured.
#[test]
fn strmenu_answers_its_default() {
    let rt = run(true);
    assert_eq!(answered(&rt, "StrMenu_Default2"), vec!["StrMenu Alpha,Beta,Gamma: 2"]);
    assert_eq!(answered(&rt, "StrMenu_NoDefault"), vec!["StrMenu Alpha,Beta,Gamma: 1"]);
    let r = result(&rt, "StrMenu_DefaultZero");
    assert!(matches!(&r.outcome, Outcome::Fail(m) if m.starts_with("Unhandled UI")), "{:?}", r.outcome);
}

/// Message and Hyperlink return and the code goes on (Y53DevRunnerProbe.al Message_NoHandler, Hyperlink_NoHandler).
#[test]
fn message_and_hyperlink_go_on() {
    let rt = run(true);
    assert_eq!(answered(&rt, "Message_NoHandler"), vec!["Message Done 7: no answer"]);
    assert_eq!(
        answered(&rt, "Hyperlink_NoHandler"),
        vec!["Hyperlink https://example.invalid/y53: no answer"]
    );
}

/// Page.RunModal(Id) answers Action::None and Page.Run(Id) returns, and no page trigger runs (Y53DevRunnerProbe.al
/// PageRunModal_NoHandler, PageRun_NoHandler). A page with an OnInit trigger was not measured.
#[test]
fn a_static_page_run_answers_without_opening_the_page() {
    let rt = run(true);
    assert_eq!(answered(&rt, "PageRunModal_NoHandler"), vec!["ModalPage 50962: Action::None"]);
    assert_eq!(answered(&rt, "PageRun_NoHandler"), vec!["Page 50962: no answer"]);
    let r = result(&rt, "PageRunModal_OnInitPage");
    assert!(matches!(&r.outcome, Outcome::Fail(m) if m.starts_with("Unhandled UI")), "{:?}", r.outcome);
}

/// A declared handler that never ran still fails the test, and a bound handler still answers
/// (Y53DevRunnerProbe.al DeclaredHandler_NotUsed, Confirm_DefaultFalse_HandlerAnswersYes).
#[test]
fn handlers_keep_their_rules() {
    let (off, on) = (run(false), run(true));
    for test in ["DeclaredHandler_NotUsed", "Confirm_HandlerStillAnswers"] {
        assert_eq!(result(&on, test).outcome, result(&off, test).outcome, "{test}");
        assert!(result(&on, test).ui_answered.is_empty(), "{test}");
    }
    assert_eq!(result(&on, "Confirm_HandlerStillAnswers").outcome, Outcome::Pass);
}
