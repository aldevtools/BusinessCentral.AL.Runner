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

table 50972 "No Client Row" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { Clustered = true; } } }

page 50974 "No Client List"
{
    PageType = List;
    SourceTable = "No Client Row";
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { } } } }
}

report 50973 "No Client Report"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "No Client Row") { } }
    requestpage { layout { area(Content) { } } }
}

report 50976 "No Client Trigger Report"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "No Client Row") { trigger OnPreDataItem() begin end; } }
    requestpage { layout { area(Content) { } } }
}

report 50977 "No Client Layout Report"
{
    dataset { dataitem(Row; "No Client Row") { column(No; "No.") { } } }
    requestpage { layout { area(Content) { } } }
}

// The reporter's cells of al-runner issue 8154 (BC 28.4, a web-service session: the test runner 130454 called over OData),
// one test per call, unchanged but for the names; and the read of al-runner issue 8158.
codeunit 50978 "No Client Web Service"
{
    Subtype = Test;

    [Test]
    [HandlerFunctions('OnConfirm')]
    procedure ConfirmHandled()
    begin
        if Confirm('Go on?') then;
    end;

    [Test]
    procedure ConfirmUnhandled()
    begin
        if Confirm('Go on?') then;
    end;

    [Test]
    [HandlerFunctions('OnStrMenu')]
    procedure StrMenuHandled()
    begin
        if StrMenu('One,Two') = 1 then;
    end;

    [Test]
    procedure StrMenuUnhandled()
    begin
        if StrMenu('One,Two') = 1 then;
    end;

    [Test]
    [HandlerFunctions('OnMessage')]
    procedure MessageHandled()
    begin
        Message('Done.');
    end;

    [Test]
    procedure MessageUnhandled()
    begin
        Message('Done.');
    end;

    [Test]
    [HandlerFunctions('OnHyperlink')]
    procedure HyperlinkHandled()
    begin
        Hyperlink('https://example.com');
    end;

    [Test]
    procedure HyperlinkUnhandled()
    begin
        Hyperlink('https://example.com');
    end;

    [Test]
    [HandlerFunctions('OnSend')]
    procedure NotificationHandled()
    var
        N: Notification;
    begin
        N.Message := 'note';
        N.Send();
    end;

    [Test]
    procedure NotificationUnhandled()
    var
        N: Notification;
    begin
        N.Message := 'note';
        N.Send();
    end;

    [Test]
    procedure ReportRunSkipsRequestPage()
    begin
        Report.Run(Report::"No Client Report");
    end;

    [Test]
    procedure ReportRunModalSkipsRequestPage()
    begin
        Report.RunModal(Report::"No Client Report");
    end;

    [Test]
    [HandlerFunctions('OnRequest')]
    procedure ReportRunWithRequestPageHandler()
    begin
        Report.Run(Report::"No Client Report");
    end;

    [Test]
    [HandlerFunctions('OnReport')]
    procedure ReportRunWithReportHandler()
    begin
        Report.Run(Report::"No Client Report");
    end;

    [Test]
    procedure ReportWithTriggerRun()
    begin
        Report.Run(Report::"No Client Trigger Report");
    end;

    [Test]
    procedure ReportWithLayoutRun()
    begin
        Report.Run(Report::"No Client Layout Report");
    end;

    [Test]
    procedure ClientTypeIsODataV4()
    begin
        if CurrentClientType() <> ClientType::ODataV4 then
            Error('client type %1', CurrentClientType());
    end;

    [Test]
    procedure PageRunById()
    begin
        asserterror Page.Run(Page::"No Client Card");
    end;

    [Test]
    procedure DialogOpen()
    var
        D: Dialog;
    begin
        D.Open('working');
        D.Close();
    end;

    [ConfirmHandler]
    procedure OnConfirm(Question: Text[1024]; var Reply: Boolean)
    begin
        Reply := true;
    end;

    [StrMenuHandler]
    procedure OnStrMenu(Options: Text[1024]; var Choice: Integer; Instruction: Text[1024])
    begin
        Choice := 1;
    end;

    [MessageHandler]
    procedure OnMessage(Msg: Text[1024])
    begin
    end;

    [HyperlinkHandler]
    procedure OnHyperlink(Link: Text[1024])
    begin
    end;

    [SendNotificationHandler]
    procedure OnSend(var N: Notification): Boolean
    begin
    end;

    [RequestPageHandler]
    procedure OnRequest(var R: TestRequestPage "No Client Report")
    begin
        R.OK().Invoke();
    end;

    [ReportHandler]
    procedure OnReport(var R: Report "No Client Report")
    begin
    end;
}

// A global TestPage: the first test would open it, the second calls a field of it.
codeunit 50975 "No Client Shared"
{
    Subtype = Test;

    var
        Shared: TestPage "No Client List";

    [Test]
    procedure OpensSharedTestPage()
    begin
        Shared.OpenView();
    end;

    [Test]
    procedure ChecksSharedTestPage()
    begin
        asserterror Shared."No.".AssertEquals('');
    end;
}

codeunit 50970 "No Client"
{
    Subtype = Test;

    [Test]
    procedure GoesToKeyInAssertError()
    var
        List: TestPage "No Client List";
    begin
        asserterror List.GoToKey('A');
    end;

    [Test]
    procedure SetsFieldInAssertError()
    var
        List: TestPage "No Client List";
    begin
        asserterror List."No.".SetValue('A');
    end;

    [Test]
    procedure OpensTestPage()
    var
        Card: TestPage "No Client Card";
    begin
        Card.OpenView();
        Card.Close();
    end;

    [Test]
    procedure OpensTestPageForEdit()
    var
        Card: TestPage "No Client Card";
    begin
        Card.OpenEdit();
        Card.Close();
    end;

    [Test]
    procedure OpensTestPageForNew()
    var
        Card: TestPage "No Client Card";
    begin
        Card.OpenNew();
        Card.Close();
    end;

    [Test]
    procedure OpensTestPageInAssertError()
    var
        Card: TestPage "No Client Card";
    begin
        asserterror Card.OpenView();
    end;

    [Test]
    procedure TrapsAndRunsPage()
    var
        Card: TestPage "No Client Card";
    begin
        Card.Trap();
        asserterror Page.Run(Page::"No Client Card");
    end;

    [Test]
    [HandlerFunctions('CardHandler')]
    procedure RunsPageWithTestPageHandler()
    begin
        asserterror Page.Run(Page::"No Client Card");
    end;

    [Test]
    [HandlerFunctions('CardModalHandler')]
    procedure RunsModalPageWithTestPageHandler()
    begin
        asserterror Page.RunModal(Page::"No Client Card");
    end;

    [Test]
    [HandlerFunctions('TypedCardHandler')]
    procedure RunsPageWithPageTypedHandler()
    begin
        asserterror Page.Run(Page::"No Client Card");
    end;

    // The reporter's repro of al-runner issue 8028, unchanged but for the names.
    [HandlerFunctions('OnRequest')]
    [Test]
    procedure RunsRequestPage()
    begin
        if Report.RunRequestPage(Report::"No Client Report") = '' then
            Error('no parameters');
    end;

    [Test]
    procedure RunsRequestPageWithoutHandler()
    begin
        if Report.RunRequestPage(Report::"No Client Report") = '' then;
    end;

    [PageHandler]
    procedure CardHandler(var Card: TestPage "No Client Card")
    begin
    end;

    [ModalPageHandler]
    procedure CardModalHandler(var Card: TestPage "No Client Card")
    begin
    end;

    [PageHandler]
    procedure TypedCardHandler(var Card: Page "No Client Card")
    begin
    end;

    [RequestPageHandler]
    procedure OnRequest(var R: TestRequestPage "No Client Report")
    begin
        R.OK().Invoke();
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

fn fails_with(rt: &CorpusRuntime, test: &str, text: &str) {
    assert_eq!(result(rt, test).outcome, Outcome::Fail(text.into()), "{test}");
}

// GuiAllowed answers false in a web-service session (al-runner issue 8028, the reporter's measurement on BC: GuiAllowed
// false, ClientType ODataV4), flagged.
#[test]
fn gui_allowed_answers_false_and_is_flagged() {
    let rt = run(true);
    assert_eq!(result(&rt, "Gui").outcome, Outcome::Pass);
    assert!(flagged(&rt, "Gui"));
}

// Al-runner issue 8154: in a web-service session the test framework calls the bound handlers of Confirm, StrMenu, Message,
// Notification.Send and Hyperlink (the reporter's cells pass on BC 28.4); each pass is flagged.
#[test]
fn the_bound_dialog_handlers_are_called_and_flagged() {
    let rt = run(true);
    for test in [
        "ConfirmHandled",
        "StrMenuHandled",
        "MessageHandled",
        "HyperlinkHandled",
        "NotificationHandled",
    ] {
        assert_eq!(result(&rt, test).outcome, Outcome::Pass, "{test}");
        assert!(flagged(&rt, test), "{test} is not flagged");
    }
}

// Al-runner issue 8154: an unhandled Message or Hyperlink fails with the test framework's "Unhandled UI" error, as with a
// client (errors/TestUnhandledUiAndDialogDefaults.al:68-107, ui-handlers/W3Hyperlink.al:65-83).
#[test]
fn an_unhandled_message_or_hyperlink_fails_with_unhandled_ui() {
    let rt = run(true);
    fails_with(&rt, "MessageUnhandled", "Unhandled UI: Message Done.");
    fails_with(&rt, "Say", "Unhandled UI: Message Done 7");
    fails_with(&rt, "HyperlinkUnhandled", "Unhandled UI: Hyperlink https://example.com");
}

// What BC does with Confirm, StrMenu and Notification.Send that no handler takes in a web-service session is not measured:
// refused (al-runner issue 8154 lists no such cell). So are Page.Run of a page id and Dialog.Open, which the measurement does
// not cover either; the page background task worker's answers no longer stand in for that session.
#[test]
fn an_unmeasured_dialog_is_not_run() {
    let rt = run(true);
    for (test, what) in [
        ("ConfirmUnhandled", "Confirm without a ConfirmHandler"),
        ("Ask", "Confirm without a ConfirmHandler"),
        ("StrMenuUnhandled", "StrMenu without a StrMenuHandler"),
        ("Menu", "StrMenu without a StrMenuHandler"),
        (
            "NotificationUnhandled",
            "Notification.Send without a SendNotificationHandler",
        ),
        ("PageRunById", "Page.Run or RunModal of page \"No Client Card\""),
        (
            "RunsPageWithPageTypedHandler",
            "Page.Run or RunModal of page \"No Client Card\"",
        ),
        ("DialogOpen", "Dialog.Open, Update or Close"),
    ] {
        refused_as(
            &rt,
            test,
            &format!("{what} in a session without a client (--no-client)"),
        );
    }
}

// Al-runner issue 8154: a processing-only report run with Report.Run or Report.RunModal skips its request page in a
// web-service session, and runs; a bound RequestPageHandler is then never called (the test fails on the handlers not
// executed); a bound ReportHandler is called.
#[test]
fn a_processing_only_report_skips_its_request_page() {
    let rt = run(true);
    for test in [
        "ReportRunSkipsRequestPage",
        "ReportRunModalSkipsRequestPage",
        "ReportRunWithReportHandler",
    ] {
        assert_eq!(result(&rt, test).outcome, Outcome::Pass, "{test}");
        assert!(flagged(&rt, test), "{test} is not flagged");
    }
    fails_with(
        &rt,
        "ReportRunWithRequestPageHandler",
        "The following UI handlers were not executed: OnRequest",
    );
    assert!(flagged(&rt, "ReportRunWithRequestPageHandler"));
}

// Neighbors of the measured report (processing-only, a requestpage section, no trigger, no layout): a report with a data item
// trigger and a report that renders are not run.
#[test]
fn a_report_other_than_the_measured_one_is_not_run() {
    let rt = run(true);
    for test in ["ReportWithTriggerRun", "ReportWithLayoutRun"] {
        refused_as(&rt, test, "in a session without a client (--no-client)");
    }
}

// Al-runner issue 8028: CurrentClientType answers ODataV4 in a web-service session (the reporter's measurement), flagged.
#[test]
fn current_client_type_answers_odata_v4_and_is_flagged() {
    let rt = run(true);
    for test in ["ClientTypeIsODataV4", "ClientType"] {
        assert_eq!(result(&rt, test).outcome, Outcome::Pass, "{test}");
        assert!(flagged(&rt, test), "{test} is not flagged");
    }
}

// The reporter's repro of al-runner issue 8158, without Permissions Mock (the reporter measured the failure with and
// without the Read grant): a read of the Session table, which the platform `System` app declares.
const SESSION_SRC: &str = r#"
codeunit 50979 "No Client Session Read"
{
    Subtype = Test;

    [Test]
    procedure SessionIsEmpty()
    var
        R: Record "Session";
    begin
        asserterror if R.IsEmpty() then;
    end;

    [Test]
    procedure SessionFindFirst()
    var
        R: Record "Session";
    begin
        asserterror R.FindFirst();
    end;

    [Test]
    procedure SessionCount()
    var
        R: Record "Session";
    begin
        if R.Count() <> 1 then Error('%1 rows', R.Count());
    end;
}
"#;

fn run_session_reads(no_client: bool) -> CorpusRuntime {
    let system = al_deps::package::read_package(&std::fs::read(al_testfiles::system_app()).unwrap())
        .expect("System.app reads")
        .1;
    let platform = al_sema::workspace::PLATFORM_APP_ID;
    CorpusRuntime::run_suites_with(
        vec![
            AppSource {
                id: "main".into(),
                name: "main".into(),
                dependencies: vec![platform.into()],
                files: vec![("main.al".into(), SESSION_SRC.into())],
            },
            AppSource {
                id: platform.into(),
                name: "System".into(),
                dependencies: vec![],
                files: system,
            },
        ],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
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

// Al-runner issue 8158: BC fails a read of the Session table in a web-service session; its error text is not in the corpus,
// so the read is refused, also inside asserterror. With a client the read finds the session's own row, as before.
#[test]
fn a_read_of_the_session_table_is_not_run() {
    let rt = run_session_reads(true);
    for test in ["SessionIsEmpty", "SessionFindFirst", "SessionCount"] {
        refused_as(
            &rt,
            test,
            "a read of the Session table in a session without a client (--no-client)",
        );
    }
    let rt = run_session_reads(false);
    let expected = "An error was expected inside an ASSERTERROR statement.";
    for test in ["SessionIsEmpty", "SessionFindFirst"] {
        assert_eq!(result(&rt, test).outcome, Outcome::Fail(expected.into()), "{test}");
    }
    assert_eq!(result(&rt, "SessionCount").outcome, Outcome::Pass);
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

fn refused_as(rt: &CorpusRuntime, test: &str, text: &str) {
    match &result(rt, test).outcome {
        Outcome::NotRun(why) => assert!(why.contains(text), "{test}: {why}"),
        other => panic!("{test}: {other:?}"),
    }
}

fn refused_as_test_page(rt: &CorpusRuntime, test: &str) {
    refused_as(
        rt,
        test,
        "in a session without a client (--no-client): BC fails it there",
    );
}

// BC fails a TestPage open in a web-service session (GuiAllowed false, ClientType ODataV4) with NotSupportedException at
// NavSession.CreateNavTestService (al-runner issue 8028, the reporter's measurement on BC). The error is not measured by
// the corpus, so each open is refused: OpenView, OpenEdit, OpenNew, Trap, also inside asserterror.
#[test]
fn a_test_page_open_is_not_run() {
    let rt = run(true);
    for test in [
        "OpensTestPage",
        "OpensTestPageForEdit",
        "OpensTestPageForNew",
        "OpensTestPageInAssertError",
        "TrapsAndRunsPage",
    ] {
        refused_as_test_page(&rt, test);
    }
}

// No TestPage can be open under --no-client (the opens and the TestPage handlers are refused), so every other call on a
// TestPage, TestPart, TestField, TestAction, TestFilter or TestRequestPage is refused too, also inside asserterror and on a
// global TestPage after the test that would have opened it: the answer "The TestPage is not open." is pinned only with a
// client (testpage/lifecycle/TestPageTeardown.al:3-24).
#[test]
fn a_call_on_a_test_page_that_is_not_an_open_is_not_run() {
    let rt = run(true);
    refused_as_test_page(&rt, "OpensSharedTestPage");
    for (test, call) in [
        ("GoesToKeyInAssertError", "TestPage.GoToKey"),
        ("SetsFieldInAssertError", "TestField.SetValue"),
        ("ChecksSharedTestPage", "TestField.AssertEquals"),
    ] {
        refused_as(
            &rt,
            test,
            &format!("{call} in a session without a client (--no-client): what BC does there is not pinned"),
        );
    }
}

// A PageHandler or ModalPageHandler that takes a TestPage is refused before the test starts (al-runner issue 8028).
#[test]
fn a_test_bound_to_a_handler_that_takes_a_test_page_is_not_run() {
    let rt = run(true);
    for test in ["RunsPageWithTestPageHandler", "RunsModalPageWithTestPageHandler"] {
        refused_as(
            &rt,
            test,
            "which takes a TestPage, in a session without a client (--no-client): BC fails a TestPage open there",
        );
    }
}

// A RequestPageHandler is bound in a web-service session (al-runner issue 8154: a bound one ends in "not executed"), so the
// reporter's repro of al-runner issue 8028 reaches Report.RunRequestPage, which stays refused with and without a handler: BC
// fails it there and its error text is not in the corpus.
#[test]
fn report_run_request_page_stays_refused() {
    let rt = run(true);
    for test in ["RunsRequestPage", "RunsRequestPageWithoutHandler"] {
        refused_as(&rt, test, "Report.RunRequestPage in a session without a client");
    }
}

#[test]
fn off_nothing_is_flagged_and_the_dialogs_end_as_before() {
    let rt = run(false);
    for test in [
        "ConfirmHandled",
        "StrMenuHandled",
        "MessageHandled",
        "HyperlinkHandled",
        "NotificationHandled",
        "NotificationUnhandled",
        "ReportRunWithRequestPageHandler",
        "ReportRunWithReportHandler",
        "DialogOpen",
    ] {
        assert_eq!(result(&rt, test).outcome, Outcome::Pass, "{test}");
        assert!(!flagged(&rt, test), "{test}");
    }
    fails_with(&rt, "ReportRunSkipsRequestPage", "Unhandled UI: RequestPage 50973");
    fails_with(&rt, "ClientTypeIsODataV4", "client type Web");
    for test in [
        "Gui",
        "Say",
        "Ask",
        "Menu",
        "ClientType",
        "ReadsWorkDate",
        "OpensTestPage",
        "OpensTestPageForEdit",
        "OpensTestPageForNew",
        "RunsRequestPage",
        "GoesToKeyInAssertError",
        "SetsFieldInAssertError",
        "OpensSharedTestPage",
        "ChecksSharedTestPage",
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
    for test in [
        "ClientType",
        "ReadsWorkDate",
        "OpensTestPage",
        "OpensTestPageForEdit",
        "RunsRequestPage",
    ] {
        assert_eq!(result(&rt, test).outcome, Outcome::Pass, "{test}");
    }
    // With a client the TestPage shapes refused under --no-client end as before: a call on a page that is not open raises
    // "The TestPage is not open." inside asserterror; the global TestPage the first test opened stays open.
    for test in [
        "GoesToKeyInAssertError",
        "SetsFieldInAssertError",
        "OpensSharedTestPage",
    ] {
        assert_eq!(result(&rt, test).outcome, Outcome::Pass, "{test}");
    }
    let expected = "An error was expected inside an ASSERTERROR statement.";
    assert_eq!(
        result(&rt, "ChecksSharedTestPage").outcome,
        Outcome::Fail(expected.into())
    );
    for test in [
        "OpensTestPageInAssertError",
        "TrapsAndRunsPage",
        "RunsPageWithTestPageHandler",
        "RunsModalPageWithTestPageHandler",
        "RunsPageWithPageTypedHandler",
    ] {
        assert_eq!(result(&rt, test).outcome, Outcome::Fail(expected.into()), "{test}");
    }
    assert_eq!(
        result(&rt, "OpensTestPageForNew").outcome,
        Outcome::Fail(
            "Sorry, the current permissions prevented the action. (Page 50971 No Client Card Insert: main)".into()
        )
    );
    assert_eq!(
        result(&rt, "RunsRequestPageWithoutHandler").outcome,
        Outcome::Fail("Unhandled UI: RequestPage 50973".into())
    );
}
