//! `--no-client` (`Session::set_clientless_session`, al-runner issue 7564): the test session is a session without a client, a
//! web-service session as the reporter of al-runner issues 8154 and 8028 measured it on BC 28.4 (assumption
//! `no-client-session`). The routines below run outside a test method (no handler can be bound): GuiAllowed answers false and
//! CurrentClientType ODataV4, flagged; a processing-only report with a requestpage section and no trigger skips its request
//! page and runs, flagged; the dialogs a page background task worker answers (BjNoGuiDialogs.al, BjNoClientDialogs.al) are
//! refused, since those answers are not measured in a web-service session. The handler paths are tested in al-testrun
//! `tests/suite/no_client_session.rs`.
use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{Built, boolean};

const SRC: &str = r#"
table 50101 "NC Row" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { Clustered = true; } } }
page 50102 "NC Dialog Page"
{
    PageType = Card;
    layout { area(Content) { field(Info; InfoText) { } } }
    var InfoText: Text;
}
page 50103 "NC Page With Trigger"
{
    PageType = Card;
    trigger OnOpenPage() begin end;
}
report 50104 "NC Report"
{
    ProcessingOnly = true;
    requestpage { layout { area(Content) { field(Flag; Flag) { } } } }
    var Flag: Boolean;
}
report 50107 "NC Plain"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "NC Row") { } }
    requestpage { layout { area(Content) { } } }
}
report 50105 "NC Hidden" { ProcessingOnly = true; UseRequestPage = false; }
report 50106 "NC Init"
{
    ProcessingOnly = true;
    requestpage { layout { area(Content) { field(Flag; Flag) { } } } }
    var Flag: Boolean;
    trigger OnInitReport() begin end;
}
codeunit 50100 "NC Probe"
{
    procedure Gui(): Boolean begin exit(GuiAllowed()); end;
    procedure Say(): Text begin Message('hello'); exit('returned'); end;
    procedure AskYes(): Boolean begin exit(Confirm('sure?', true)); end;
    procedure AskNo(): Boolean begin exit(Confirm('sure?', false)); end;
    procedure Menu(): Integer begin exit(StrMenu('One,Two', 1, 'pick one')); end;
    procedure MenuPlain(): Integer begin exit(StrMenu('One,Two', 2)); end;
    procedure ModalById(): Text begin exit(Format(Page.RunModal(Page::"NC Dialog Page"))); end;
    procedure RunById() begin Page.Run(Page::"NC Dialog Page"); end;
    procedure ModalOnVariable(): Text var P: Page "NC Dialog Page"; begin exit(Format(P.RunModal())); end;
    procedure RunOnVariable() var P: Page "NC Dialog Page"; begin P.Run(); end;
    procedure ModalWithRecord() var R: Record "NC Row"; begin Page.RunModal(Page::"NC Dialog Page", R); end;
    procedure ModalWithTrigger() begin Page.RunModal(Page::"NC Page With Trigger"); end;
    procedure Progress(): Text var D: Dialog; begin D.Open('working #1######'); D.Update(1, 5); D.Close(); exit('done'); end;
    procedure OpenOnly(): Text var D: Dialog; begin D.Open('open only'); exit('open returned'); end;
    procedure Link(): Text begin Hyperlink('https://example.com'); exit('ran'); end;
    procedure Notify(): Boolean var N: Notification; begin N.Message := 'note'; exit(N.Send()); end;
    procedure Recall() var N: Notification; begin N.Message := 'note'; N.Recall(); end;
    procedure ReportModal(): Text begin Report.RunModal(Report::"NC Plain"); exit('ran'); end;
    procedure ReportRun(): Text begin Report.Run(Report::"NC Plain"); exit('ran'); end;
    procedure ReportWithControl() begin Report.Run(Report::"NC Report"); end;
    procedure ReportWithWindow() begin Report.Run(Report::"NC Plain", true); end;
    procedure ReportHidden(): Text begin Report.Run(Report::"NC Hidden"); exit('ran'); end;
    procedure ReportInit() begin Report.Run(Report::"NC Init"); end;
    procedure ReportOnVariable() var R: Report "NC Report"; begin R.Run(); end;
    procedure FilterPage(): Boolean var F: FilterPageBuilder; begin F.AddTable('Rows', Database::"NC Row"); exit(F.RunModal()); end;
    procedure Upload(): Boolean var S: InStream; Name: Text; begin exit(UploadIntoStream('upload', '', '', Name, S)); end;
    procedure Download() var S: InStream; Name: Text; begin DownloadFromStream(S, '', '', '', Name); end;
    procedure SessionUpdate(): Text var S: SessionSettings; begin S.Init(); S.RequestSessionUpdate(false); exit('ran'); end;
    procedure Kind(): Text begin exit(Format(CurrentClientType())); end;
}
"#;

fn on() -> Built {
    let mut b = Built::from_source(SRC);
    b.session.set_clientless_session(true);
    b
}

/// Runs `routine` without a client; true when the run marked `no-client-session`.
fn run_on(routine: &str) -> (Result<Value, Signal>, bool) {
    let mut b = on();
    clear_hits();
    let r = b.run(routine);
    (r, take_hits().contains(&Assumption::NoClientSession))
}

fn returned_text(routine: &str, want: &str) {
    let (r, flagged) = run_on(routine);
    assert!(matches!(&r, Ok(Value::Text(t)) if &**t == want), "{routine}: {r:?}");
    assert!(flagged, "{routine} is not flagged");
}

fn refused(routine: &str) {
    let (r, flagged) = run_on(routine);
    match r {
        Err(Signal::Unsupported(m)) => assert!(m.contains("session without a client"), "{routine}: {m}"),
        other => panic!("{routine}: {other:?}"),
    }
    assert!(!flagged, "{routine}: a refusal is flagged");
}

#[test]
fn gui_allowed_answers_false_without_a_client() {
    let (r, flagged) = run_on("Gui");
    assert!(matches!(r, Ok(Value::Boolean(false))), "{r:?}");
    assert!(flagged);
}

// Al-runner issue 8028: CurrentClientType answers ODataV4 in a web-service session.
#[test]
fn current_client_type_answers_odata_v4() {
    returned_text("Kind", "ODataV4");
}

// Al-runner issue 8154: a processing-only report with an empty request page skips it in a web-service session and runs. A
// report whose request page has a control, or a call with a RequestWindow argument, is refused.
#[test]
fn a_processing_only_report_skips_its_request_page() {
    returned_text("ReportRun", "ran");
    returned_text("ReportModal", "ran");
}

#[test]
fn a_report_run_that_shows_no_page_goes_on_and_an_unprobed_one_is_refused() {
    let (r, flagged) = run_on("ReportHidden");
    assert!(matches!(&r, Ok(Value::Text(t)) if &**t == "ran"), "{r:?}");
    assert!(!flagged);
    for routine in ["ReportInit", "ReportOnVariable", "ReportWithControl", "ReportWithWindow"] {
        refused(routine);
    }
}

// The page background task worker's answers do not stand in for a web-service session: Confirm and StrMenu without a
// handler, a page run, Dialog.Open, Notification.Send without a handler, FilterPageBuilder.RunModal, UploadIntoStream and
// RequestSessionUpdate are refused, as are the calls the worker does not answer either.
#[test]
fn a_dialog_the_measurement_does_not_cover_is_refused() {
    for routine in [
        "AskYes",
        "AskNo",
        "Menu",
        "MenuPlain",
        "ModalById",
        "ModalOnVariable",
        "RunById",
        "RunOnVariable",
        "ModalWithRecord",
        "ModalWithTrigger",
        "Progress",
        "OpenOnly",
        "Notify",
        "FilterPage",
        "Upload",
        "SessionUpdate",
        "Recall",
        "Download",
    ] {
        refused(routine);
    }
}

// Outside a test method a Message and a Hyperlink have no handler to reach, as with a client: Message is refused, Hyperlink
// raises Unhandled UI (session_fns.rs `hyperlink`).
#[test]
fn message_and_hyperlink_outside_a_test_method_end_as_with_a_client() {
    let (r, _) = run_on("Say");
    assert!(matches!(&r, Err(Signal::Unsupported(m)) if m.contains("outside a test method")), "{r:?}");
    let (r, _) = run_on("Link");
    assert!(matches!(&r, Err(Signal::Error(e)) if e.message == "Unhandled UI: Hyperlink https://example.com"), "{r:?}");
}

#[test]
fn a_session_with_a_client_is_unchanged() {
    let mut b = Built::from_source(SRC);
    clear_hits();
    assert!(boolean(b.run("Gui")));
    // Outside a test method a dialog has no handler to reach: refused as before the option existed.
    for routine in ["Say", "Menu", "ModalById", "ReportRun"] {
        match b.run(routine) {
            Err(Signal::Unsupported(m)) => assert!(!m.contains("session without a client"), "{routine}: {m}"),
            Err(Signal::Error(e)) => assert_ne!(e.code, "CallbackNotAllowed", "{routine}"),
            other => panic!("{routine}: {other:?}"),
        }
    }
    assert!(!take_hits().contains(&Assumption::NoClientSession));
}
