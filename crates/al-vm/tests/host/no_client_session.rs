//! `--no-client` (`Session::set_clientless_session`, al-runner issue 7564): the test session is a session without a client.
//! Pins: corpus tests/al-language/pages/background-tasks/BjNoGuiDialogs.al:8-10 (GuiAllowed false; Message returns and
//! reaches no MessageHandler; Confirm raises CallbackNotAllowed whatever its default). Whether a web-service session answers
//! as that worker does is the assumption `no-client-session`; every other dialog is refused.

use al_builtins::native::Signal;
use al_testkit::{Built, boolean};

const SRC: &str = r#"
codeunit 50100 "NC Probe"
{
    procedure Gui(): Boolean begin exit(GuiAllowed()); end;
    procedure Say(): Text begin Message('hello'); exit('returned'); end;
    procedure AskYes(): Boolean begin exit(Confirm('sure?', true)); end;
    procedure AskNo(): Boolean begin exit(Confirm('sure?', false)); end;
    procedure Menu(): Integer begin exit(StrMenu('a,b', 1, 'pick')); end;
    procedure Link() begin Hyperlink('https://example.com'); end;
    procedure Kind(): Text begin exit(Format(CurrentClientType())); end;
}
"#;

fn on() -> Built {
    let mut b = Built::from_source(SRC);
    b.session.set_clientless_session(true);
    b
}

#[test]
fn gui_allowed_answers_false_without_a_client() {
    assert!(!boolean(on().run("Gui")));
}

#[test]
fn message_returns_without_an_error_without_a_client() {
    let r = on().run("Say");
    assert!(
        matches!(r, Ok(al_builtins::value::Value::Text(ref t)) if &**t == "returned"),
        "{r:?}"
    );
}

#[test]
fn confirm_raises_callback_not_allowed_whatever_its_default() {
    for routine in ["AskYes", "AskNo"] {
        match on().run(routine) {
            Err(Signal::Error(e)) => {
                assert_eq!(e.code, "CallbackNotAllowed", "{routine}");
                assert!(
                    e.message.contains("show a confirmation dialog box: sure?"),
                    "{}",
                    e.message
                );
            }
            other => panic!("{routine}: {other:?}"),
        }
    }
}

#[test]
fn a_dialog_nothing_pins_is_refused_without_a_client() {
    for routine in ["Menu", "Link", "Kind"] {
        match on().run(routine) {
            Err(Signal::Unsupported(m)) => assert!(m.contains("session without a client"), "{routine}: {m}"),
            other => panic!("{routine}: {other:?}"),
        }
    }
}

#[test]
fn a_session_with_a_client_is_unchanged() {
    let mut b = Built::from_source(SRC);
    assert!(boolean(b.run("Gui")));
    // Outside a test method a Message has no handler to reach: refused as before the option existed.
    match b.run("Say") {
        Err(Signal::Unsupported(m)) => assert!(!m.contains("session without a client"), "{m}"),
        other => panic!("{other:?}"),
    }
}

const REPORT_SRC: &str = r#"
report 50110 "NC Shown" { ProcessingOnly = true; }
report 50111 "NC Hidden" { ProcessingOnly = true; UseRequestPage = false; }
codeunit 50112 "NC Reports"
{
    procedure Shown() begin Report.Run(Report::"NC Shown"); end;
    procedure Hidden() begin Report.Run(Report::"NC Hidden"); end;
}
"#;

#[test]
fn a_report_run_is_refused_only_when_its_request_page_would_open() {
    let mut b = Built::from_source(REPORT_SRC);
    b.session.set_clientless_session(true);
    match b.run("Shown") {
        Err(Signal::Unsupported(m)) => assert!(
            m.contains("request page") && m.contains("session without a client"),
            "{m}"
        ),
        other => panic!("{other:?}"),
    }
    let r = b.run("Hidden");
    assert!(
        !matches!(&r, Err(Signal::Unsupported(m)) if m.contains("session without a client")),
        "{r:?}"
    );
}
