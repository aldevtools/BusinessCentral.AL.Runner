TransactionTypeReadOnlyWrites => {
    id: "transaction-type-read-only-writes",
    assumed: "At a Database.CurrentTransactionType of Snapshot, Browse or Report, an Insert, Modify, Delete, DeleteAll (also of no row) or ModifyAll of a record that is not temporary raises DB:TransActionIsReadOnly (\"Write operations in the database (INSERT, DELETE, and MODIFY) are not allowed for the selected transaction type.\"), the row stays as it was and the type stays. Measured in the code of a test, outside a report, a Codeunit.Run, a TryFunction and a page; there and for Rename, Truncate, a CRM table and a system table the write is refused. (The text and code are pinned for a report's own TransactionType: reports/W3ReportTransactions.al:207-249.)",
    tests: "Code that sets Snapshot, Browse or Report for its reads and then writes before it sets the type back.",
    probe: "record/TxTypeSetAfterCall.al TxS16 to TxS34 of corpus pull request PROBE_PR, green on a local Business Central container.",
    awaiting: Awaiting {
        issues: &[8707],
        files: &["tests/al-language/record/TxTypeSetAfterCall.al"],
        since: "2026-11-22",
    },
}
