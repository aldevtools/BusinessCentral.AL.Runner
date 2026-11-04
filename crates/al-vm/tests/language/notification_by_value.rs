//! A Notification passed by value is the caller's own notification (branch overload-regression).
//! Microsoft's page "Check Credit Limit" fills the notification's data in a procedure that takes it by value
//! (`PopulateDataOnNotification(CreditLimitNotification: Notification)`, CheckCreditLimit.Page.al:543), and Tests-ERM
//! ERM Sales Documents II SendNotificationHandler (ERMSalesDocumentsII.Codeunit.al:6705) reads that data back.
//! Assignment still copies (errors/errorinfo/EnTNotification.al:Copy_Semantics).

use al_builtins::value::Value;

fn run(body: &str) -> String {
    let src = format!(
        "codeunit 50101 Filler {{ procedure Fill(N: Notification) begin N.SetData('k', 'v'); end; }}
         codeunit 50100 C {{ procedure F(): Text var N: Notification; B: Notification; P: Codeunit Filler; begin {body} end; }}"
    );
    match al_testkit::run_in(&src, "C", "F") {
        Ok(Value::Text(t) | Value::Code(t)) => t.to_string(),
        Err(al_builtins::native::Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        other => format!("{other:?}"),
    }
}

#[test]
fn data_set_on_a_notification_passed_by_value_reaches_the_caller() {
    assert_eq!(run("N.Message('m'); P.Fill(N); exit(Format(N.HasData('k')));"), "Yes");
}

#[test]
fn a_by_value_notification_handed_on_by_reference_fills_the_callers() {
    let src = "codeunit 50101 Filler { procedure Fill(N: Notification) begin Inner(N); end; procedure Inner(var N: Notification) begin N.SetData('k', 'v'); end; }
         codeunit 50100 C { procedure F(): Text var N: Notification; P: Codeunit Filler; begin N.Message('m'); P.Fill(N); exit(Format(N.HasData('k'))); end; }";
    match al_testkit::run_in(src, "C", "F") {
        Ok(Value::Text(t) | Value::Code(t)) => assert_eq!(t.to_string(), "Yes"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_assignment_still_copies_the_notification() {
    assert_eq!(
        run("N.Message('m'); B := N; P.Fill(B); exit(Format(N.HasData('k')));"),
        "No"
    );
}

fn run_page(body: &str) -> String {
    let src = format!(
        "page 50102 Filler {{ PageType = Card; procedure Fill(N: Notification) begin Inner(N); end; procedure Inner(var N: Notification) begin N.SetData('k', 'v'); end; }}
         codeunit 50100 C {{ procedure F(): Text var N: Notification; P: Page Filler; begin {body} end; }}"
    );
    match al_testkit::run_in(&src, "C", "F") {
        Ok(Value::Text(t) | Value::Code(t)) => t.to_string(),
        Err(al_builtins::native::Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        other => format!("{other:?}"),
    }
}

#[test]
fn a_page_procedure_that_hands_its_by_value_notification_on_by_reference_fills_the_callers() {
    // CheckCreditLimit.Page.al:543-547 into CreditLimitDetails.Page.al:125.
    assert_eq!(
        run_page("N.Message('m'); P.Fill(N); exit(Format(N.HasData('k')));"),
        "Yes"
    );
}
