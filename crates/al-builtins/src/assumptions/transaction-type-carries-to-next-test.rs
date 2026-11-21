TransactionTypeCarriesToNextTest => {
    id: "transaction-type-carries-to-next-test",
    assumed: "A test method starts with the Database.CurrentTransactionType the test method before it in the codeunit left, not UpdateNoLocks: Browse, Snapshot and Report as well, and an Update report at the start of the test after one that left Update enters no transaction world. Marked where AL code reads such a carried type (a CurrentTransactionType read or set, an Update report run without its request page). (Update carried into a test that commits and writes is pinned: session-system/TestClearAllTransactionTypeContracts.al:257-286.)",
    tests: "No Microsoft test needs it; Microsoft's libraries set the type back after they change it (Library - Manufacturing.RefreshProdOrder). The outside reporter's repro (al-runner issue 8606).",
    probe: "session-system/TxTypeNextTestMethod.al of corpus pull request 1488, TxN01 to TxN12 (Browse, Snapshot, Report and Update left by a set after a Commit, Browse with a read after the set, Snapshot a Commit kept; the type at the next test's start, and an Update report there), green on a local Business Central container. A test the runner refused while it ran leaves a type that is not known: reading it is refused.",
    awaiting: Awaiting {
        issues: &[8606],
        files: &["tests/al-language/session-system/TxTypeNextTestMethod.al"],
        since: "2026-11-21",
    },
}
