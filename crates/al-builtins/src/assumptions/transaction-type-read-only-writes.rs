TransactionTypeReadOnlyWrites => {
    id: "transaction-type-read-only-writes",
    assumed: "At a Database.CurrentTransactionType of Snapshot, Browse or Report, an Insert, Modify, Delete, DeleteAll (also of no row) or ModifyAll of a record that is not temporary raises DB:TransActionIsReadOnly (\"Write operations in the database (INSERT, DELETE, and MODIFY) are not allowed for the selected transaction type.\"), the row stays as it was and the type stays. Measured in the code of a test (outside a report, a Codeunit.Run, a TryFunction and a page, not under TransactionModel None), in the statement form, without RunTrigger, on an application table with no AutoIncrement field, no subscriber of its database events and no global database trigger, so no AL code runs for the call before the write: an Insert of a key that is not stored, a Modify that changes a field of a stored row, a Delete of a stored row, a DeleteAll and a ModifyAll of at least one row. Every other write there is refused (a duplicate key, a Modify that changes nothing, a ModifyAll of no row, RunTrigger, a subscriber or a trigger that runs first, Rename, Truncate, a RecordRef, a CRM table, a system table). (The text and code are pinned for a report's own TransactionType: reports/W3ReportTransactions.al:207-249.)",
    tests: "Code that sets Snapshot, Browse or Report for its reads and then writes before it sets the type back.",
    probe: "record/TxTypeSetAfterCall.al TxS16 to TxS34 of corpus pull request 1520, green on a local Business Central container and in the corpus pull request's CI on every supported major.",
    awaiting: Awaiting {
        issues: &[8707],
        files: &["tests/al-language/record/TxTypeSetAfterCall.al"],
        since: "2026-11-22",
    },
}
