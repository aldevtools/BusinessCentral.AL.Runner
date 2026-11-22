TransactionTypeSetAfterStart => {
    id: "transaction-type-set-after-start",
    assumed: "Database.CurrentTransactionType set once the transaction has started, by a write or, in a test codeunit's first test method, by the test runner before a Commit or a rollback: a set to the type the transaction has has no effect (after a read too); at UpdateNoLocks a set to Update or Snapshot raises DB:CannotChangeTransactionType (\"... changed from UpdateNoLocks to Snapshot.\") and a set to Browse or Report has no effect; at Update no set has an effect. Marked where the set runs, and where AL code reads the type it left. A set to another type after a read that is not the test runner's is refused: whether the read started the transaction depends on the server's data cache. (After a write at UpdateNoLocks a set to Update raises and a set to Report has no effect: session-system/ProbeSystemEnumMembers.al:114-141; after a write at Update a set to Snapshot has no effect: session-system/TestClearAllTransactionTypeContracts.al:274-286.)",
    tests: "Test codeunits whose first test method sets the type before its first Commit (Library - Manufacturing.RefreshProdOrder commits first); partner tests that set Snapshot or Browse after a write.",
    probe: "record/TxTypeSetAfterCall.al TxS01 to TxS15 and session-system/TxTypeFirstTestMethod.al (A tests after nothing and after a read) of corpus pull request PROBE_PR, green on a local Business Central container.",
    awaiting: Awaiting {
        issues: &[8708, 8709],
        files: &["tests/al-language/record/TxTypeSetAfterCall.al", "tests/al-language/session-system/TxTypeFirstTestMethod.al"],
        since: "2026-11-22",
    },
}
