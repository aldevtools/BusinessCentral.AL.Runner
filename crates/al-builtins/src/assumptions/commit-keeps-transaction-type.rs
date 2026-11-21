CommitKeepsTransactionType => {
    id: "commit-keeps-transaction-type",
    assumed: "A Commit keeps Database.CurrentTransactionType: the transaction after it has the type the transaction before it had, UpdateNoLocks, Snapshot, Browse or Report as well as Update. Marked where AL code reads a type a Commit kept other than Update (a CurrentTransactionType read or set, an Update report run without its request page). (A Commit at Update reads Update after it: session-system/ProbeSystemEnumMembers.al:94-110.)",
    tests: "Library - Manufacturing.RefreshProdOrder reads the type right after its first Commit and sets it back after its second (al-runner issue 8605).",
    probe: "record/TxTypeUpdateReport.al of corpus pull request 1488, cells TxU21 to TxU26 and TxU34 to TxU38 (a type set at the test's start, an Insert, a FindFirst or nothing, one or two Commits, then the type), TxU07, TxU08, TxU19 and TxU20 (an Update report after a Commit at UpdateNoLocks).",
    awaiting: Awaiting {
        issues: &[8605],
        files: &["tests/al-language/record/TxTypeUpdateReport.al"],
        since: "2026-11-21",
    },
}
