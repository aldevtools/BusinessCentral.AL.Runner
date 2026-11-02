GlobalTriggerMaskBits => {
    id: "global-trigger-mask-bits",
    assumed: "A subscriber of Global Triggers GetGlobalTableTriggerMask may answer a mask other than 0 and 15: bit 8 raises OnGlobalRename (before OnDatabaseRename), no other bit raises a global event.",
    tests: "No Microsoft test: an outside report (al-runner issue 6644) whose install roots run with subscribers of apps installed earlier; those subscribers add mask bits.",
    probe: "install/lt-y79-install-unrelated/Y79InstallUnrelated.al (Y79_TestSession_Table61370..61379, Y79_PerCompany_*) on corpus branch al-runtime-rs-staging-y79, measured on the three supported majors; not in the runner's pin yet.",
    awaiting: Awaiting {
        issues: &[6644],
        files: &["tests/al-language/install/lt-y79-install-unrelated/Y79InstallUnrelated.al"],
        since: "2026-11-02",
    },
}
