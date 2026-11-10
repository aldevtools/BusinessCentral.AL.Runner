//! `--no-client` (`Session::set_clientless_session`, al-runner issue 7564): the test session is a session without a client.
//! Every answer is the one a page background task worker gives. Pins: corpus tests/al-language/pages/background-tasks/
//! BjNoGuiDialogs.al:8-10 (GuiAllowed false; Message returns and reaches no MessageHandler; Confirm raises
//! CallbackNotAllowed whatever its default) and BjNoClientDialogs.al, RunPageBackgroundTask_Dialogs (the steps `01` to `14`
//! named below; corpus pull request 1286, BC 27.5, 28.5 and 29.0). Whether a web-service session answers as that worker
//! does is the assumption `no-client-session`; a dialog or a call shape the probe does not cover is refused.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{Built, boolean};

const SERVER: &str = "Microsoft Dynamics 365 Business Central Server";

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
    procedure ReportModal() begin Report.RunModal(Report::"NC Report"); end;
    procedure ReportRun() begin Report.Run(Report::"NC Report"); end;
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

fn returned_boolean(routine: &str, want: bool) {
    let (r, flagged) = run_on(routine);
    assert!(matches!(&r, Ok(Value::Boolean(b)) if *b == want), "{routine}: {r:?}");
    assert!(flagged, "{routine} is not flagged");
}

/// `routine` raises CallbackNotAllowed for the callback `what`, with the worker's text.
fn callback_not_allowed(routine: &str, what: &str) {
    let (r, flagged) = run_on(routine);
    match r {
        Err(Signal::Error(e)) => {
            assert_eq!(e.code, "CallbackNotAllowed", "{routine}");
            assert_eq!(
                e.message,
                format!(
                    "{SERVER} attempted to issue a client callback to {what}. Client callbacks are not supported on {SERVER}."
                ),
                "{routine}"
            );
        }
        other => panic!("{routine}: {other:?}"),
    }
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
    returned_boolean("Gui", false);
}

#[test]
fn message_returns_without_an_error_without_a_client() {
    returned_text("Say", "returned");
}

// BjNoGuiDialogs.al:9-10; the worker's text names the Server (BjNoGuiDialogs.al:204).
#[test]
fn confirm_raises_callback_not_allowed_whatever_its_default() {
    for routine in ["AskYes", "AskNo"] {
        callback_not_allowed(
            routine,
            "show a confirmation dialog box: sure? (CodeUnit 50100 NC Probe)",
        );
    }
}

// BjNoClientDialogs.al `01 StrMenu with instruction`, `02 StrMenu without instruction` (two spaces after the colon).
#[test]
fn strmenu_raises_callback_not_allowed_with_and_without_an_instruction() {
    callback_not_allowed("Menu", "show a selection menu: pick one (CodeUnit 50100 NC Probe)");
    callback_not_allowed("MenuPlain", "show a selection menu:  (CodeUnit 50100 NC Probe)");
}

// BjNoClientDialogs.al `03 Page.RunModal by id`, `13 Page.RunModal on a variable`, `04 Page.Run by id`.
#[test]
fn a_page_run_raises_callback_not_allowed() {
    callback_not_allowed("ModalById", "run page 50102 NC Dialog Page as modal");
    callback_not_allowed("ModalOnVariable", "run page 50102 NC Dialog Page as modal");
    callback_not_allowed("RunById", "run page 50102 NC Dialog Page");
}

#[test]
fn a_page_run_shape_the_probe_does_not_cover_is_refused() {
    for routine in ["RunOnVariable", "ModalWithRecord", "ModalWithTrigger"] {
        refused(routine);
    }
}

// BjNoClientDialogs.al `08 Report.RunModal`, `09 Report.Run`.
#[test]
fn a_report_run_whose_request_page_would_open_raises_callback_not_allowed() {
    callback_not_allowed("ReportModal", "run page 50104 RequestOptionsPage as modal");
    callback_not_allowed("ReportRun", "run page 50104 RequestOptionsPage as modal");
}

#[test]
fn a_report_run_that_shows_no_page_goes_on_and_an_unprobed_one_is_refused() {
    let (r, flagged) = run_on("ReportHidden");
    assert!(matches!(&r, Ok(Value::Text(t)) if &**t == "ran"), "{r:?}");
    assert!(!flagged);
    for routine in ["ReportInit", "ReportOnVariable"] {
        refused(routine);
    }
}

// BjNoClientDialogs.al `05 Dialog Open Update Close`, `14 Dialog Open only`, `06 Hyperlink`, `07 Notification.Send`,
// `10 FilterPageBuilder.RunModal`, `11 UploadIntoStream`, `12 SessionSettings.RequestSessionUpdate`.
#[test]
fn the_dialogs_that_return_in_the_worker_return() {
    returned_text("Progress", "done");
    returned_text("OpenOnly", "open returned");
    returned_text("Link", "ran");
    returned_boolean("Notify", true);
    returned_boolean("FilterPage", false);
    returned_boolean("Upload", false);
    returned_text("SessionUpdate", "ran");
}

#[test]
fn a_dialog_the_probe_does_not_cover_is_refused() {
    for routine in ["Recall", "Download", "Kind"] {
        refused(routine);
    }
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
