RecallNeverSentRunsHandler => {
    id: "recall-never-sent-runs-handler",
    assumed: "Notification.Recall of a notification that was given an Id and never sent runs the bound [RecallNotificationHandler] once, handed that notification. What Recall answers then is not assumed: a use of the answer is refused.",
    tests: "Microsoft's Tests-SCM codeunit \"SCM Orders IV\" PurchaseLineBlankTypeDifferentPayToVendorNo: [HandlerFunctions('YesConfirmHandler,RecallNotificationHandler')], and only Purchase Header.RecallModifyAddressNotification (a fresh Notification, Id := a fixed Guid, Recall, no Send) reaches the recall handler; it passes in Microsoft's CI. Also al-runner issue 6924 item 3.3 (two partner tests).",
    probe: "ui-handlers/NotificationRecallNeverSent.al on corpus branch al-runtime-rs-staging-r6924b: Recall_IdSetNeverSent_HandlerBound, _IdOnlyNeverSent_HandlerBound, _NoIdNeverSent_HandlerBound, _IdSetNeverSent_NoHandler, _OtherVariableWithIdOfSent_HandlerBound and _NoHandler.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/ui-handlers/NotificationRecallNeverSent.al"],
        since: "2026-11-04",
    },
}
