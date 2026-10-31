StartSessionRecordHandedOver => {
    id: "startsession-record-handed-over",
    assumed: "Under Disabled test isolation, `StartSession` with a temporary record hands the new session a temporary record holding a copy of all the caller's rows, standing on the caller's current row; a record with filters (standard filter group) or marks with MarkedOnly hands them over; the caller keeps its rows.",
    tests: "No Microsoft test: a public partner app that hands a filtered or temporary record to StartSession (33 tests). Measured on the newest major by corpus writer Y30 (SaaS sandbox, Microsoft's runner 130451).",
    probe: "Y30StartSessionIsolDisabled.al on branch al-runtime-rs-probe-y30-session (76761008): Y30_StartSession_TemporaryRecord, _RecordWithFilters, _RecordWithMarks, _UnsavedRecord. Not runnable by the corpus CI until a leg runs a test app under a runner with TestIsolation Disabled.",
    awaiting: Awaiting {
        issues: &[],
        files: &["tests/al-language/session/lt-y30-startsession-isol-disabled/Y30StartSessionIsolDisabled.al"],
        since: "2026-10-31",
    },
}
