TransactionTypeCarriesToNextTest => {
    id: "transaction-type-carries-to-next-test",
    assumed: "A test method starts with the Database.CurrentTransactionType the test method before it in the codeunit left, not UpdateNoLocks: Browse, Snapshot and Report as well, and an Update report at the start of the test after one that left Update enters no transaction world. Marked where AL code reads such a carried type, Update as well (a CurrentTransactionType read or set, an Update report run without its request page), until a set that takes effect or a Commit at Update (Update carried into a test that commits and writes is pinned: session-system/TestClearAllTransactionTypeContracts.al:274-286). Only a type a test method of the same codeunit left carries: the first test method starts at UpdateNoLocks whatever install code left (TestClearAllTransactionTypeContracts.al:12-13), and a set to another type at its start, before a Commit or a rollback, is refused (whether the test runner's transaction has started there depends on the runner and on what ran before it: the description of corpus pull request 1520).",
    tests: "No Microsoft test needs it; Microsoft's libraries set the type back after they change it (Library - Manufacturing.RefreshProdOrder). The outside reporter's repro (al-runner issue 8606).",
    probe: "session-system/TxTypeNextTestMethod.al of corpus pull request 1488, TxN01 to TxN12 (Browse, Snapshot, Report and Update left by a set after a Commit, Browse with a read after the set, Snapshot a Commit kept; the type at the next test's start, and an Update report there), green on a local Business Central container and in the corpus pull request's CI on every supported major. A test that did not run to its end here (refused before or while it ran, a failure not trusted, a step of the test runner around it) whose reach can set the type, or is not fully known, leaves a type that is not known: reading it is refused until a set that takes effect.",
    awaiting: Awaiting {
        issues: &[8606],
        files: &["tests/al-language/session-system/TxTypeNextTestMethod.al"],
        since: "2026-11-21",
    },
}
