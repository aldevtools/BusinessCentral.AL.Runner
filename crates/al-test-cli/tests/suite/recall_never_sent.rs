//! al-runner issue 6924, item 3.3: `Notification.Recall()` of a notification this variable never sent, with a
//! `[RecallNotificationHandler]` bound. Base Application gives a fresh Notification a fixed Id and recalls it without a
//! Send (Purchase Header.RecallModifyAddressNotification); Microsoft's Tests-SCM "SCM Orders IV"
//! PurchaseLineBlankTypeDifferentPayToVendorNo binds a recall handler for it. Pinned on BC 27.5 by
//! ui-handlers/NotificationRecallNeverSent.al:41-145 (corpus PR 1119).

use al_test_cli::report::Outcome;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    al_testkit::outcomes_in(&format!("codeunit 63110 T {{ Subtype = Test; {tests} }}"), "T")
}

// Recall_IdSetNeverSent_HandlerBound: `recall` Yes, the handler runs at each Recall with the notification's message.
#[test]
fn recall_of_an_id_never_sent_runs_the_bound_handler_and_answers_true() {
    let o = outcomes(
        "var Log: Text;
        [Test] [HandlerFunctions('RecallH')]
        procedure FixedIdNeverSent() var N: Notification; begin
            N.Id := '{8E0B1E64-1D5B-4C8B-9E55-5F1D0C0A6671}'; N.Message('m');
            if not N.Recall() then Error('first'); if not N.Recall() then Error('second');
            if Log <> 'r=m;r=m;' then Error('log %1', Log); end;
        [RecallNotificationHandler] procedure RecallH(var N: Notification): Boolean begin Log += 'r=' + N.Message() + ';'; exit(true); end;",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// Recall_NoIdNeverSent_HandlerBound: `recall` No and the handler does not run (an optional handler, so the test passes).
#[test]
fn recall_of_a_notification_without_id_never_sent_answers_false_and_runs_no_handler() {
    let o = outcomes(
        "var Log: Text;
        [Test] [HandlerFunctions('RecallH')]
        procedure NoId() var N: Notification; begin N.Message('x');
            if N.Recall() then Error('true'); if Log <> '' then Error('log %1', Log); end;
        [RecallNotificationHandler(true)] procedure RecallH(var N: Notification): Boolean begin Log += 'r;'; exit(true); end;",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}
