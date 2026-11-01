StaleOwnInsertTwoCommitsRowWritten => {
    id: "stale-own-insert-two-commits-row-written",
    assumed: "A stale buffer from the variable's own Insert two or three Commits back, made stale by another variable's Modify in the transaction the last Commit closed, is written whole (the stale buffer wins, OnModify runs once with xRec the buffer) when the only writes of the transaction after the last Commit are Modifies of that same row through other variables and it took no lock.",
    tests: "No Microsoft test: al-runner issue 6522, the reporter's repro passed on real BC (both tests); a partner app's test holds a Production BOM Header from its own Insert. Measured by corpus writer Y27 (Y27StaleOtherVarTests.al on branch al-runtime-rs-staging-y27, SaaS sandbox, 15/15).",
    probe: "Y27StaleOtherVarTests.al Y27_2C_* and Y27_3C_*: the stored row and the buffer after the stale Modify.",
    awaiting: Awaiting {
        issues: &[6522],
        files: &["tests/al-language/record/lt-sb-stale-own-modify/Y27StaleOtherVarTests.al"],
        since: "2026-10-31",
    },
}
