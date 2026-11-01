StartSessionTimeoutNotReached => {
    id: "startsession-timeout-not-reached",
    assumed: "Under Disabled test isolation, `StartSession(.., Company, Record, Timeout)` with a session that ends before its Timeout runs as the overload without a Timeout. (A session that still runs at its Timeout is stopped there by BC while the caller runs on: measured, and refused, as a session that runs alongside the caller is not modelled.)",
    tests: "No Microsoft test: a public partner app that passes a session timeout to StartSession (3 tests). Measured on the newest major by corpus writer Y30 only for a session that outlives its Timeout.",
    probe: "Y30StartSessionIsolDisabled.al Y30_StartSession_Timeout on branch al-runtime-rs-probe-y30-session (76761008); wanted in addition: a session that ends before its Timeout (its rows committed, IsSessionActive false after it).",
    awaiting: Awaiting {
        issues: &[],
        files: &["tests/al-language/session/lt-y30-startsession-isol-disabled/Y30StartSessionIsolDisabled.al"],
        since: "2026-10-31",
    },
}
