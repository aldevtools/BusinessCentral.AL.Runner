UpdateReportBySessionType => {
    id: "update-report-by-session-type",
    assumed: "A TransactionType = Update report that a report variable runs (Run or RunModal) without its request page, under TransactionModel AutoCommit, outside a TryFunction, a Codeunit.Run and a lock, enters no transaction world while the session's transaction type is Update, and a world for every other type: after a Commit with one CurrentTransactionType(Update) set and then any read and write calls (measured: nothing, Get, FindFirst, FindSet and Next, Count and IsEmpty, Insert, Modify, Delete, a Get and an Insert; two runs in a row with nothing between them); after CurrentTransactionType(Update) at the test's start and then nothing, an Insert or a FindFirst; after a Commit at UpdateNoLocks with no set, or with one set to UpdateNoLocks, and then nothing or an Insert (a world: the Insert refuses the run); after a Commit with one set to Snapshot, Report or Browse and nothing else (a world). Inside an asserterror only the world shapes. Every other moment is refused. (The start of the test method with no set is pinned: record/TestTxModelReportRun.al:177-211, record/TestTxModelReportExec.al:137-171, :213-257.)",
    tests: "Library - Manufacturing.RefreshProdOrder (Commit, CurrentTransactionType(Update), Get, RunModal of report 99001025 Refresh Production Order without its request page, Commit): Microsoft's manufacturing tests and the outside reporter's partner tests that create production orders (al-runner issue 8324); a partner test's own refresh (Commit, then RunModal of report 99001025 without its request page).",
    probe: "record/TxTypeUpdateReport.al of corpus pull request 1488, cells TxU01 to TxU20 and TxU27 to TxU33 (green on a local Business Central container, and in the corpus pull request's CI on every supported major).",
    awaiting: Awaiting {
        issues: &[8324],
        files: &["tests/al-language/record/TxTypeUpdateReport.al"],
        since: "2026-11-21",
    },
}
