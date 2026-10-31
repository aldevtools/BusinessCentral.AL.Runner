StaleDeleteEventBeforeRecordChanged => {
    id: "stale-delete-event-before-record-changed",
    assumed: "A Delete through a stale buffer from the variable's own Insert two or three Commits back, on a table with an OnDelete trigger or delete event subscribers: when the new transaction wrote nothing, the OnBeforeDeleteEvent subscribers run, then (Delete(true)) the OnDelete trigger, then DB:RecordChanged, the row stays and OnAfterDeleteEvent never runs; when another variable Modified that row in the new transaction, the row is deleted with the order of a fresh Delete.",
    tests: "No Microsoft test: al-runner issue 6546, the reporter's repro passed on real BC (both tests); a partner app's test deletes its own ledger entry with Delete(false) through the variable that inserted it. Measured by corpus writer Y32 (Y32StaleDeleteTests.al on branch al-runtime-rs-staging-y32, SaaS sandbox).",
    probe: "Y32StaleDeleteTests.al Y32_2C_* and Y32_3C_*: the error code and text, the order of the trigger and the events, and the stored row.",
    awaiting: Awaiting {
        issues: &[6546],
        files: &["tests/al-language/record/lt-sb-stale-own-modify/Y32StaleDeleteTests.al"],
        since: "2026-10-31",
    },
}
