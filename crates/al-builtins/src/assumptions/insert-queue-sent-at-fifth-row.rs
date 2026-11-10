InsertQueueSentAtFifthRow => {
    id: "insert-queue-sent-at-fifth-row",
    assumed: "The statement-form Inserts of a table without a Blob or AutoIncrement field are queued, and the queue is sent with its 5th row, a Commit and the next access to the table; a duplicate key that is pending when the queue is sent raises at the Insert that sent it. Observed on BC 27.5, 28.5 and 29.0 for a duplicate as the 2nd to the 7th insert, with and without a Commit, and for the reporter's shapes (the same four or five keys twice); not measured: what other statements do to the queue besides the reads and writes of the table, a table with several companies in play, the After event of the Insert that sends the queue (refused).",
    tests: "No corpus test on the pinned master yet. The probe is corpus pull request 1279 (record/duplicate-insert/Y7407DuplicateInsertBuffer.al), observed on BC 27.5, 28.5 and 29.0 and pinned on that branch.",
    probe: "record/duplicate-insert/Y7407DuplicateInsertBuffer.al; still to measure: a Modify, Delete or guarded Insert of the table between the queued rows, two tables interleaved, OnAfterInsert subscribers on the Insert that sends the queue.",
    awaiting: Awaiting {
        issues: &[7407],
        files: &["tests/al-language/record/duplicate-insert/Y7407DuplicateInsertBuffer.al"],
        since: "2026-11-10",
    },
}
