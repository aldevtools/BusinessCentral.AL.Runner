//! Gaps found by running Microsoft's BaseApp test buckets (branch bucket-misc): notifications,
//! and the refusals that stay. AL source → compile → run as a test codeunit. Each test names the
//! corpus test it mirrors.

use al_test_cli::report::Outcome;

fn outcomes_with(objects: &str, tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{objects}\ncodeunit 63100 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    outcomes_with("", tests)
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

const HELPERS: &str = "procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;";

// ---------------------------------------------------------------- Notification

// handlers/TestNotificationHandler.al:15-26: Send goes to the [SendNotificationHandler], which
// gets the notification with its message; the handler answers true.
#[test]
fn send_goes_to_the_send_notification_handler() {
    passes(&format!(
        "{HELPERS}
        var Calls: Integer; Saw: Text;
        [Test] [HandlerFunctions('Handler')]
        procedure Sent() var N: Notification; begin
            Calls := 0; N.Id := CreateGuid(); N.Message('Test notification'); N.Scope(NotificationScope::LocalScope);
            N.Send(); Check('1', Format(Calls)); Check('Test notification', Saw); end;
        [SendNotificationHandler] procedure Handler(var Notif: Notification): Boolean begin
            Calls += 1; Saw := Notif.Message(); exit(true); end;"
    ));
}

// handlers/TestNotificationHandler.al:60-72: Recall after a handled Send does not throw, with
// only a send handler bound.
#[test]
fn recall_after_a_handled_send_does_not_throw() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Handler')]
        procedure Recalled() var N: Notification; begin
            N.Id := CreateGuid(); N.Message('Message to recall'); N.Scope(NotificationScope::LocalScope);
            N.Send(); N.Recall(); end;
        [SendNotificationHandler] procedure Handler(var Notif: Notification): Boolean begin exit(true); end;"
    ));
}

// al-language/pages/lt-ca-odd-667/CaNotificationTests.al:14-38 (handlers at :61, :70): a [SendNotificationHandler(true)] or [RecallNotificationHandler(true)]
// that never runs lets the test pass (HandlerIsOptional).
#[test]
fn an_optional_notification_handler_that_never_fires_passes() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('OptSend,OptRecall')]
        procedure NothingSent() begin end;
        [SendNotificationHandler(true)] procedure OptSend(var Notif: Notification): Boolean begin exit(true); end;
        [RecallNotificationHandler(true)] procedure OptRecall(var Notif: Notification): Boolean begin exit(true); end;"
    ));
}

// A declared handler that never fired fails the test (handlers/TestPageQueryCloseMessageHandler_Tests.al:51).
#[test]
fn a_send_notification_handler_that_never_fires_fails_the_test() {
    let o = outcomes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Handler')]
        procedure Quiet() begin end;
        [SendNotificationHandler] procedure Handler(var Notif: Notification): Boolean begin exit(true); end;"
    ));
    assert!(matches!(&o[0].1, Outcome::Fail(_)), "{o:?}");
}

// handlers/TestNotificationSendRecallContracts.al:59-75, 123-131 (BC 27.5): a notification nobody set has an
// empty message, LocalScope, no data and the null Guid for an id; Send without a handler shows nothing, raises
// nothing, returns true and gives it an Id; Recall of a sent notification returns true, of one never sent false.
#[test]
fn a_fresh_notification_has_empty_defaults_and_send_gives_it_an_id() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Defaults() var N: Notification; Null: Guid; begin
            Check('', N.Message()); Check('LocalScope', Format(N.Scope())); Check('No', Format(N.HasData('k')));
            Check('', N.GetData('k')); Check('Yes', Format(N.Id() = Null));
            N.Message('no handler');
            Check('Yes', Format(N.Send())); Check('No', Format(N.Id() = Null));
            Check('Yes', Format(N.Recall())); end;
        [Test]
        procedure NeverSent() var N: Notification; begin N.Message('never sent'); Check('No', Format(N.Recall())); end;
        [Test]
        procedure NeverSentWithAnId() var N: Notification; begin N.Id := CreateGuid(); Check('No', Format(N.Recall())); end;"
    ));
}

// :77-93: a handler that answers false still gets true back from Send, it sees the sender's message, scope and data,
// and its change to the message is seen by the sender afterwards.
#[test]
fn send_returns_true_for_a_handler_answering_false_and_the_sender_sees_the_change() {
    passes(&format!(
        "{HELPERS}
        var Log: Text;
        [Test] [HandlerFunctions('Handler')]
        procedure False1() var N: Notification; begin
            Log := ''; N.Message('hello'); N.SetData('k', 'v');
            Check('Yes', Format(N.Send()));
            Check('send msg=hello data=v;', Log); Check('changed', N.Message()); end;
        [SendNotificationHandler] procedure Handler(var Notif: Notification): Boolean begin
            Log += 'send msg=' + Notif.Message() + ' data=' + Notif.GetData('k') + ';'; Notif.Message('changed'); exit(false); end;"
    ));
}

// :95-109: Recall runs the [RecallNotificationHandler] with the sender's current notification; :111-121: Recall after
// a Send with no recall handler returns true.
#[test]
fn recall_runs_the_recall_handler_with_the_current_notification() {
    passes(&format!(
        "{HELPERS}
        var Log: Text;
        [Test] [HandlerFunctions('SendH,RecallH')]
        procedure Both() var N: Notification; begin
            Log := ''; N.Message('hello'); Check('Yes', Format(N.Send())); Check('changed too', N.Message());
            Check('Yes', Format(N.Recall())); Check('send msg=hello;recall msg=changed too;', Log); end;
        [Test] [HandlerFunctions('SendH')]
        procedure NoRecallHandler() var N: Notification; begin N.Message('hello'); N.Send(); Check('Yes', Format(N.Recall())); end;
        [SendNotificationHandler] procedure SendH(var Notif: Notification): Boolean begin
            Log += 'send msg=' + Notif.Message() + ';'; Notif.Message('changed too'); exit(true); end;
        [RecallNotificationHandler] procedure RecallH(var Notif: Notification): Boolean begin
            Log += 'recall msg=' + Notif.Message() + ';'; exit(true); end;"
    ));
}

// handlers/ProbeNotificationRecallEdges.al (BC 27.5): every Recall of a sent notification runs the recall handler and
// answers true, whatever the handler answers; a Send whose handler answered false is a Send all the same.
#[test]
fn recall_runs_the_handler_each_time_and_answers_true() {
    passes(&format!(
        "{HELPERS}
        var Log: Text;
        [Test] [HandlerFunctions('SendTrue,RecallFalse')]
        procedure Twice() var N: Notification; begin
            Log := ''; N.Message('m'); N.Send(); Check('Yes', Format(N.Recall())); Check('Yes', Format(N.Recall()));
            Check('send;recallFalse;recallFalse;', Log); end;
        [Test] [HandlerFunctions('SendFalse,RecallTrue')]
        procedure AfterAFalseSend() var N: Notification; begin
            Log := ''; N.Message('m'); Check('Yes', Format(N.Send())); Check('Yes', Format(N.Recall()));
            Check('sendFalse;recallTrue;', Log); end;
        [SendNotificationHandler] procedure SendTrue(var Notif: Notification): Boolean begin Log += 'send;'; exit(true); end;
        [SendNotificationHandler] procedure SendFalse(var Notif: Notification): Boolean begin Log += 'sendFalse;'; exit(false); end;
        [RecallNotificationHandler] procedure RecallFalse(var Notif: Notification): Boolean begin Log += 'recallFalse;'; exit(false); end;
        [RecallNotificationHandler] procedure RecallTrue(var Notif: Notification): Boolean begin Log += 'recallTrue;'; exit(true); end;"
    ));
}
