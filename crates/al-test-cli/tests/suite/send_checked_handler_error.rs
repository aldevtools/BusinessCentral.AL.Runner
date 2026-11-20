//! al-runner issue 8341: an error raised in a `[SendNotificationHandler]` while the sender uses the result of
//! `Notification.Send()` (`if N.Send() then;`). The reporter measured on BC 29.1 that the error does not reach the
//! caller; what `Send` then returns is not measured and no corpus test pins either. The runner refuses that shape. An
//! error in a checked Send of a notification with no message is refused for the same reason. The unchecked forms keep
//! raising.

use al_test_cli::report::Outcome;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    al_testkit::outcomes_in(&format!("codeunit 63120 T {{ Subtype = Test; {tests} }}"), "T")
}

fn one(tests: &str) -> Outcome {
    outcomes(tests).remove(0).1
}

// The reporter's HandlerErrorInsideCheckedSend (issue 8341), reduced to one codeunit.
#[test]
fn a_handler_error_in_a_checked_send_is_refused() {
    let o = one("[Test] [HandlerFunctions('ExpectSecond')]
        procedure Checked() var First: Notification; Second: Notification; begin
            First.Message('first'); First.Scope := NotificationScope::LocalScope; if First.Send() then;
            Second.Message('second'); Second.Scope := NotificationScope::LocalScope; if Second.Send() then; end;
        [SendNotificationHandler] procedure ExpectSecond(var N: Notification): Boolean begin
            if N.Message() <> 'second' then Error('wrong text %1', N.Message()); end;");
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("Notification.Send")),
        "{o:?}"
    );
}

// The same error with the result read into a variable.
#[test]
fn a_handler_error_in_a_send_whose_result_is_assigned_is_refused() {
    let o = one("[Test] [HandlerFunctions('Fails')]
        procedure Assigned() var N: Notification; B: Boolean; begin N.Message('m'); B := N.Send(); end;
        [SendNotificationHandler] procedure Fails(var N: Notification): Boolean begin Error('handler failed'); end;");
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("Notification.Send")),
        "{o:?}"
    );
}

// The reporter's HandlerErrorInsideUncheckedSend: main's result, the error reaches the caller.
#[test]
fn a_handler_error_in_an_unchecked_send_still_reaches_the_caller() {
    let o = one("[Test] [HandlerFunctions('Fails')]
        procedure Unchecked() var N: Notification; begin
            N.Message('m'); asserterror N.Send();
            if GetLastErrorText() <> 'handler failed' then Error('got %1', GetLastErrorText()); end;
        [SendNotificationHandler] procedure Fails(var N: Notification): Boolean begin Error('handler failed'); end;");
    assert_eq!(o, Outcome::Pass);
}

// A checked Send whose handler raises nothing answers true (ui-handlers/TestNotificationSendRecallContracts.al:89).
#[test]
fn a_checked_send_whose_handler_raises_nothing_answers_true() {
    let o = one("[Test] [HandlerFunctions('QuietH')]
        procedure Quiet() var N: Notification; begin N.Message('m'); if not N.Send() then Error('false'); end;
        [SendNotificationHandler] procedure QuietH(var N: Notification): Boolean begin exit(false); end;");
    assert_eq!(o, Outcome::Pass);
}

// A checked Send of a notification with no message: main raised the unchecked form's error; no pin says what the
// checked form does.
#[test]
fn a_checked_send_without_a_message_is_refused() {
    let o = one("[Test] procedure Empty() var N: Notification; begin if N.Send() then; end;");
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("Notification.Send")),
        "{o:?}"
    );
}

// The unchecked form keeps its error (errors/errorinfo/EnTNotification.al:284).
#[test]
fn an_unchecked_send_without_a_message_still_raises() {
    let o = one(
        "[Test] procedure Empty() var N: Notification; begin asserterror N.Send();
            if GetLastErrorText() <> 'Message cannot be empty when sending a Notification.' then Error('got %1', GetLastErrorText()); end;",
    );
    assert_eq!(o, Outcome::Pass);
}
