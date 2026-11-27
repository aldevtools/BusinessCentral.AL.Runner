RowVersionCheckedInsert => {
    id: "row-version-checked-insert",
    assumed: "A checked Insert (its return value used) of a Record into a database table whose statement-form Insert is queued sends its row at once: Database.LastUsedRowVersion becomes that row's SystemRowVersion, greater than before. Only while no LockTable ran in the open transaction and the Insert's table held no other unsent inserted row; other tables may hold unsent rows. Every other checked Insert is refused (al-vm records/stale.rs `send_checked_insert`).",
    tests: "al-runner issue 8671 (an incremental export suite that inserts with `if not Rec.Insert() then Error(...)` and then stores LastUsedRowVersion() + 1 as an upper bound). Corpus PR 1621 (aldevtools/BusinessCentral.AL.Language.Tests, open), session-system/row-version-writes/RowVersionWriteShapes.al, observed on a local Business Central on the newest corpus major: RowVersion_CheckedInsert asserts `last/prev` greater and `last/row` same after each of two checked Inserts, and the next Insert's SystemRowVersion is LastUsedRowVersion() + 1.",
    probe: "session-system/row-version-writes/RowVersionWriteShapes.al of corpus PR 1621 (open), not yet pinned; the older corpus majors are measured by its CI. Only relations are measured, never a number. Still to measure: a checked Insert while its table holds unsent inserted rows, a checked Insert through a RecordRef, after LockTable, into a table with a Blob or AutoIncrement field, and under a non-default test isolation.",
    awaiting: Awaiting {
        issues: &[8671],
        files: &["tests/al-language/session-system/row-version-writes/RowVersionWriteShapes.al"],
        since: "2026-11-27",
    },
}
