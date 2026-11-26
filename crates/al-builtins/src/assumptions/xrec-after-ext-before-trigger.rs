XRecAfterExtBeforeTrigger => {
    id: "xrec-after-ext-before-trigger",
    assumed: "In a table's own `OnInsert` or `OnDelete`, run by `Insert(true)` or `Delete(true)` on a table with exactly one table extension that declares `OnBeforeInsert` or `OnBeforeDelete`, xRec's fields hold the record as it was at the call, before the `OnBeforeInsertEvent` or `OnBeforeDeleteEvent` subscribers and the extension's trigger ran, and xRec's filters and current key are the ones Rec has after the extension's trigger (al-vm `records/ext_trigger_xrec.rs`). Measured only on a normal record of the current company in a table with one table extension that declares the trigger, where the caller's record had no filters, no marks, the primary key and filter group 0 at the call. A call is flagged only when some code of the table or of its extensions names xRec (`table_code_names_xrec`); no other code can read it. Refused in that call shape (every read of that xRec refuses): the extension's trigger left a filter group other than 0, set a filter in a filter group other than 0, or moved Rec to another row (a Get or Find in the trigger). Main's answer (the buffer after the subscribers, before the extension's trigger), not covered: every other call shape, among them a temporary record, two or more table extensions, filters, marks or another key on the caller's record, and a table extension that declares neither trigger. xRec in the extension's own triggers stays refused (`XREC_OF_EXT_INSERT_DELETE_TRIGGER`).",
    tests: "No Microsoft test. An outside report needs xRec around a table extension's insert triggers (al-runner issue 8986); the follow-up is al-runner issue 9185.",
    probe: "Corpus PR 1611 (ready, green on every supported major), record/events/XebXRecAfterExtBeforeTrigger.al:292-874: 17 cells of Insert(true) and Delete(true) with the extension or a subscriber changing a field, a filter or the key.",
    awaiting: Awaiting {
        issues: &[8986, 9185],
        files: &["tests/al-language/record/events/XebXRecAfterExtBeforeTrigger.al"],
        since: "2026-11-26",
    },
}
