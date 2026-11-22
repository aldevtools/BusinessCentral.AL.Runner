RowVersionWriteAfterInsert => {
    id: "row-version-write-after-insert",
    assumed: "Database.LastUsedRowVersion in the open transaction grows past the value read before a write in these shapes, and an Insert's row version counts once a later write of its table sends it. A Delete, DeleteAll or Truncate of the one table whose inserted rows no read sent yet: the value becomes their row version. A Rename or ModifyAll of that table, or of a table the open transaction inserted into and a read already sent: the value becomes the last row version the write handed out, and a renamed row's SystemRowVersion equals it. Every other write after an Insert keeps the refusal (al-vm records/stale.rs `row_version_after_write`).",
    tests: "al-runner issue 8671 (an incremental export suite that stores LastUsedRowVersion as a watermark after inserting its test rows). Corpus PR 1509 (aldevtools/BusinessCentral.AL.Language.Tests, merged, not pinned yet), session-system/row-version-writes/RowVersionWrites.al, observed on a local Business Central 27.5 and 29.0 for the final assertions (28.5 on an earlier commit of the same cells): DeleteAfterInsert, DeleteAllAfterInsert, TruncateAfterInsert, RenameAfterInsert, ModifyAllAfterInsert, RenameAfterFlushedInsert and ModifyAllAfterFlushedInsert assert that the value is greater after the write.",
    probe: "session-system/row-version-writes/RowVersionWrites.al of corpus PR 1509 (merged), not yet pinned. Only relations are measured, never a number, and that the value stays the same is not measured (another session can write between two reads). Still to measure: a write after Inserts into two tables that no read sent, a Rename or ModifyAll of a table the transaction did not insert into, a Rename that rewrites related rows.",
    awaiting: Awaiting {
        issues: &[8671],
        files: &["tests/al-language/session-system/row-version-writes/RowVersionWrites.al"],
        since: "2026-11-22",
    },
}
