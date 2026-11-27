RowVersionDeleteAll => {
    id: "row-version-delete-all",
    assumed: "A Record DeleteAll() with no argument, not through a RecordRef and without MarkedOnly, whose rows no trigger or event wrote through a Record (no write noted inside it), leaves Database.LastUsedRowVersion as it was when no inserted row of the open transaction is unsent, and the later writes of the transaction move it as without the DeleteAll. When the inserted rows nothing sent yet are all of the DeleteAll's table, it sends them first: the value becomes the last of their row versions, also when its filter matches none of them; a filtered one that deletes one of them is refused. Every other DeleteAll and every Truncate stays refused (al-vm records/stale.rs `row_version_after_write`, `last_used_row_version`, `end_filtered_delete_all`).",
    tests: "al-runner issue 8671 (an incremental export suite that deletes the rows of two tables, one filtered, then inserts and reads LastUsedRowVersion()). Corpus PR 1621 (aldevtools/BusinessCentral.AL.Language.Tests, open), session-system/row-version-writes/RowVersionWriteShapes.al, observed on a local Business Central on the newest corpus major: RowVersion_DeleteAllNoInsert_Unfiltered, _Filtered and _EmptyTable assert `last/prev` same and the next Insert's SystemRowVersion LastUsedRowVersion() + 1; RowVersion_DeleteAllAfterInsert_Unfiltered, _FilteredOtherRows and _CommittedRowsToo assert `last/prev` greater and `last/row` same as the last inserted row; RowVersion_IssueWriteOrder runs two DeleteAlls before the Inserts and Modifies it measures.",
    probe: "session-system/row-version-writes/RowVersionWriteShapes.al of corpus PR 1621 (open), not yet pinned; the older corpus majors are measured by its CI. Only relations are measured, never a number. Still to measure: DeleteAll(true) with an OnDelete trigger, a DeleteAll through a RecordRef, a DeleteAll while another table holds unsent inserted rows, and Truncate.",
    awaiting: Awaiting {
        issues: &[8671],
        files: &["tests/al-language/session-system/row-version-writes/RowVersionWriteShapes.al"],
        since: "2026-11-27",
    },
}
