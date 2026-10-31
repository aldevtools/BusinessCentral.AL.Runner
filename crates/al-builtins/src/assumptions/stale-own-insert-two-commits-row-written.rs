StaleOwnInsertTwoCommitsRowWritten => {
    id: "stale-own-insert-two-commits-row-written",
    assumed: "A stale buffer from the variable's own Insert two Commits back (another variable modified the row and committed in between) is written whole, the stale buffer winning, when the only writes of the transaction after the last Commit are Modifies of that same row through other variables and it took no lock.",
    tests: "No Microsoft test: al-runner issue 6522, the reporter's repro passed on real BC 29 (both tests); a partner app's test holds a Production BOM Header from its own Insert. The Base App shape is consumption posting (Mfg. Item Jnl.-Post Line modifies the Prod. Order Component through its own variable).",
    probe: "The reporter's repro with the stored row read back after the Modify: the stale copy whole, or merged with the new transaction's write; and the same at one and three Commits.",
    awaiting: Awaiting {
        issues: &[6522],
        files: &["tests/al-language/record/lt-y27/Y27StaleInsertRowWritten.al"],
        since: "2026-10-31",
    },
}
