TransactionTypeSetAfterStart => {
    id: "transaction-type-set-after-start",
    assumed: "Database.CurrentTransactionType set once a write has started the transaction: a set to the type the transaction has has no effect (after a write at UpdateNoLocks or Update this is the runner's answer before the probe; after a read, TxS11 to TxS15, it rests on the probe and is marked); at UpdateNoLocks a set to Update or Snapshot raises DB:CannotChangeTransactionType (\"... changed from UpdateNoLocks to Snapshot.\") and a set to Browse or Report has no effect; at Update no set has an effect. Marked where the set runs, and where AL code reads the type it left. A set to another type after a read is refused (whether the read started the transaction depends on the server's data cache), and so is one at the start of a test codeunit's first test method before a Commit or a rollback (whether the test runner's transaction has started there depends on the runner and on what ran before it). (After a write at UpdateNoLocks a set to Update raises and a set to Report has no effect: session-system/ProbeSystemEnumMembers.al:114-141; a set to Snapshot raises and a set to Browse or Report has no effect: metadata/platform-tables/PlTDatabase.al:143-171; after a write at Update a set to Snapshot has no effect: session-system/TestClearAllTransactionTypeContracts.al:274-286. So the entry is what a set to UpdateNoLocks, Browse or Report does after a write at Update.)",
    tests: "Code that sets the type after it has written, before a Commit.",
    probe: "record/TxTypeSetAfterCall.al TxS01 to TxS15 of corpus pull request 1520, green on a local Business Central container and in the corpus pull request's CI on every supported major.",
    awaiting: Awaiting {
        issues: &[8708, 8709],
        files: &["tests/al-language/record/TxTypeSetAfterCall.al"],
        since: "2026-11-22",
    },
}
