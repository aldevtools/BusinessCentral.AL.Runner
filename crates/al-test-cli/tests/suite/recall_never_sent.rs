//! al-runner issue 6924, item 3.3: `Notification.Recall()` of a notification this variable never sent, with a
//! `[RecallNotificationHandler]` bound. Base Application gives a fresh Notification a fixed Id and recalls it without a
//! Send (Purchase Header.RecallModifyAddressNotification); Microsoft's test binds a recall handler for that call.

use al_test_cli::report::Outcome;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    al_testkit::outcomes_in(&format!("codeunit 63110 T {{ Subtype = Test; {tests} }}"), "T")
}

// Microsoft's Tests-SCM "SCM Orders IV".PurchaseLineBlankTypeDifferentPayToVendorNo passes on real BC with a
// non-optional [RecallNotificationHandler] that only Purchase Header.RecallModifyAddressNotification reaches: a fresh
// Notification with a fixed Id, recalled without a Send.
#[test]
fn recall_of_a_fixed_id_never_sent_runs_the_bound_recall_handler() {
    let o = outcomes(
        "var Log: Text;
        [Test] [HandlerFunctions('RecallH')]
        procedure FixedIdNeverSent() var N: Notification; begin
            N.Id := '{8E0B1E64-1D5B-4C8B-9E55-5F1D0C0A6671}'; N.Recall();
            if Log <> 'r;' then Error('log %1', Log); end;
        [RecallNotificationHandler] procedure RecallH(var N: Notification): Boolean begin Log += 'r;'; exit(true); end;",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// What Recall answers for such a notification is not pinned: a use of the answer is refused, never guessed.
#[test]
fn the_answer_of_recall_of_a_fixed_id_never_sent_is_refused() {
    let o = outcomes(
        "[Test] [HandlerFunctions('RecallH')]
        procedure Used() var N: Notification; begin
            N.Id := '{8E0B1E64-1D5B-4C8B-9E55-5F1D0C0A6672}'; if N.Recall() then; end;
        [RecallNotificationHandler] procedure RecallH(var N: Notification): Boolean begin exit(true); end;
        [Test]
        procedure UsedNoHandler() var N: Notification; begin N.Id := CreateGuid(); if N.Recall() then Error('true'); end;",
    );
    assert!(matches!(&o[0].1, Outcome::NotRun(m) if m.contains("Recall")), "{o:?}");
    assert!(matches!(&o[1].1, Outcome::NotRun(m) if m.contains("Recall")), "{o:?}");
}

// A notification with no Id that was never sent, with a recall handler bound: whether the handler runs is not pinned
// (the corpus pins only the answer false, with no handler bound). Refused, not failed.
#[test]
fn recall_of_a_notification_without_id_never_sent_with_a_handler_is_refused() {
    let o = outcomes(
        "[Test] [HandlerFunctions('RecallH')]
        procedure NoId() var N: Notification; begin N.Message('x'); N.Recall(); end;
        [RecallNotificationHandler] procedure RecallH(var N: Notification): Boolean begin exit(true); end;",
    );
    assert!(matches!(&o[0].1, Outcome::NotRun(m) if m.contains("Recall")), "{o:?}");
}
