InsertQueueSentAtFifthRow => {
    id: "insert-queue-sent-at-fifth-row",
    assumed: "The statement-form Inserts AL code makes (on a Record or a RecordRef) into a table without a Blob or AutoIncrement field are queued, and the queue is sent with its 5th row, a Commit and the next access to the table; a duplicate key that is pending when the queue is sent raises at the Insert that sent it. Observed on all three majors of the matrix for a duplicate as the 2nd to the 7th insert, with and without a Commit, and for the reporter's shapes (the same four or five keys twice); not measured: what other statements do to the queue besides the reads and writes of the table, a table with several companies in play, the After event of the Insert that sends the queue (refused), a valid row that sends the queue while a duplicate is pending (refused: in every measured cell the Insert that sends it is itself a duplicate), an Insert the platform makes itself, such as an XmlPort import (kept out of the queue).",
    tests: "No corpus test on the pinned master yet. The probe is corpus pull request 1279 (record/duplicate-insert/Y7407DuplicateInsertBuffer.al), observed on all three majors of the matrix and pinned on that branch.",
    probe: "record/duplicate-insert/Y7407DuplicateInsertBuffer.al; still to measure: a Modify, Delete or guarded Insert of the table between the queued rows, two tables interleaved, OnAfterInsert subscribers on the Insert that sends the queue, a valid row as the 5th queued row with a duplicate pending (K1, K2, K1, K3, K4; inside a TryFunction: does K4 stay), an XmlPort import of five or more rows with a duplicate key among them, RecordRef.Insert.",
    awaiting: Awaiting {
        issues: &[7407],
        files: &["tests/al-language/record/duplicate-insert/Y7407DuplicateInsertBuffer.al"],
        since: "2026-11-10",
    },
}
