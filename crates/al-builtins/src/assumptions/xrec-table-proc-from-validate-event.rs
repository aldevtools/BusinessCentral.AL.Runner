XrecTableProcFromValidateEvent => {
    id: "xrec-table-proc-from-validate-event",
    assumed: "A procedure of a table or of one of its table extensions, called on Rec by a subscriber of the OnBeforeValidateEvent or OnAfterValidateEvent of that table, runs with the xRec of the Validate that raised the event, shared with the subscriber's xRec parameter: when the subscriber, of a codeunit not bound manually, declares Rec by `var` (and, when it declares xRec and CurrFieldNo, xRec by `var`), its frame is the one the event put right above the code that called the Validate, and the Validate is the one `Record.Validate` AL code called on the record, on any buffer. A procedure that the shared procedure reaches through other code (a codeunit it hands Rec to) keeps main's plain call. Every other call keeps main's answer (al-vm records/validate_event_xrec.rs).",
    tests: "al-runner issue 8306: the reporter's repro passes on real BC (an OnAfterValidateEvent subscriber calling a table procedure and a table extension procedure on Rec, after an Insert); the issue has the code. Corpus PR 1438 ran green on a local Business Central container.",
    probe: "record/validate/XRecTableProcThroughSubscriber.al (corpus PR 1438): an OnAfterValidateEvent subscriber calling a table extension procedure and a table procedure on Rec after Insert, after Get, after FindFirst, on a fresh and on a temporary record; a subscriber that does not declare xRec; the table extension procedure assigning xRec, read in the subscriber's xRec parameter after the call; an OnBeforeValidateEvent subscriber after Insert.",
    awaiting: Awaiting {
        issues: &[8306],
        files: &["tests/al-language/record/validate/XRecTableProcThroughSubscriber.al"],
        since: "2026-11-18",
    },
}
