GetUrlCurrentIsWeb => {
    id: "geturl-current-is-web",
    assumed: "GetUrl(ClientType::Current) in a test session answers what GetUrl(ClientType::Web) answers, alone and with a company, an object and a record. Measured on real BC (three majors) by corpus branch al-runtime-rs-staging-p6924c; the entry goes when that file is in the pinned corpus.",
    tests: "Microsoft and partner code that asks GetUrl(ClientType::Current) for a link (al-runner issue 6924).",
    probe: "session-system/session/GetUrlShape.al GU6924_Current_EqualsWhichForm on corpus branch al-runtime-rs-staging-p6924c.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/session-system/session/GetUrlShape.al::GU6924_Current_EqualsWhichForm"],
        since: "2026-11-04",
    },
}
