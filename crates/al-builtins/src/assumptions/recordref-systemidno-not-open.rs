RecordRefSystemIdNoNotOpen => {
    id: "recordref-systemidno-not-open",
    assumed: "`RecordRef.SystemIdNo()` on a RecordRef that is not open (never opened, closed, cleared) answers 2000000000 without an error.",
    tests: "No Microsoft test: a public partner app's test that adds the system fields of a table through a RecordRef (1 test). Measured on the newest major by corpus writer Y30 (W4RecordRefGetUnopened.al Y30_SystemIdNo_NotOpen, SaaS sandbox).",
    probe: "W4RecordRefGetUnopened.al Y30_SystemIdNo_NotOpen on branch al-runtime-rs-staging-y30 (2f7609a1): SystemIdNo never opened, open, after Close, after Clear.",
    awaiting: Awaiting {
        issues: &[],
        files: &["tests/al-language/recordref/W4RecordRefGetUnopened.al"],
        since: "2026-10-31",
    },
}
