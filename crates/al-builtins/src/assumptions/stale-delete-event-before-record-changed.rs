StaleDeleteEventBeforeRecordChanged => {
    id: "stale-delete-event-before-record-changed",
    assumed: "A Delete(false) through a stale buffer that a Modify through it would refuse with DB:RecordChanged, on a table whose only delete hooks are OnBeforeDeleteEvent subscribers (an OnDelete trigger does not run under Delete(false)), runs the OnBeforeDeleteEvent subscribers and then raises DB:RecordChanged; the row stays.",
    tests: "No Microsoft test: al-runner issue 6546, the reporter's repro passed on real BC (both tests); a partner app's test deletes its own ledger entry with Delete(false) through the variable that inserted it. Corpus writer Y32 measures it.",
    probe: "Y32's file in record/lt-sb-stale-own-modify: the event order, the error code and text and the stored row of a stale Delete(false) and Delete(true) with and without OnBeforeDeleteEvent and OnAfterDeleteEvent subscribers.",
    awaiting: Awaiting {
        issues: &[6546],
        files: &["tests/al-language/record/lt-sb-stale-own-modify/Y32StaleDeleteTests.al"],
        since: "2026-10-31",
    },
}
