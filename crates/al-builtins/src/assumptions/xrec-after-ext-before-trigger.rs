XRecAfterExtBeforeTrigger => {
    id: "xrec-after-ext-before-trigger",
    assumed: "In a table's own `OnInsert` or `OnDelete`, run by `Insert(true)` or `Delete(true)` on a table with exactly one table extension that declares `OnBeforeInsert` or `OnBeforeDelete`, xRec's fields hold the record as it was at the call, before the `OnBeforeInsertEvent` or `OnBeforeDeleteEvent` subscribers and the extension's trigger ran, and xRec's filters and current key are the ones Rec has after the extension's trigger (al-vm `records/ext_trigger_xrec.rs`). Measured only on a normal record of the current company that had no filters, the primary key and filter group 0 at the call, where the extension's trigger left filter group 0 and did not move Rec to another row. Refused (every read of that xRec refuses): every other shape, among them a temporary record, two or more table extensions, filters or another key on the caller's record, a filter group other than 0 after the trigger, and a Get or Find in the trigger. Not covered, main's answer: a table extension that declares neither trigger. xRec in the extension's own triggers stays refused (`XREC_OF_EXT_INSERT_DELETE_TRIGGER`).",
    tests: "No Microsoft test. An outside report needs xRec around a table extension's insert triggers (al-runner issue 8986); the follow-up is al-runner issue 9185.",
    probe: "Corpus PR 1611 (ready, green on 27.5, 28.5, 29.0), record/events/XebXRecAfterExtBeforeTrigger.al:292-874: 17 cells of Insert(true) and Delete(true) with the extension or a subscriber changing a field, a filter or the key.",
    awaiting: Awaiting {
        issues: &[9185],
        files: &["tests/al-language/record/events/XebXRecAfterExtBeforeTrigger.al"],
        since: "2026-11-26",
    },
}
