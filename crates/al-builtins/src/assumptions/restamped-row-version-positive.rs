RestampedRowVersionPositive => {
    id: "restamped-row-version-positive",
    assumed: "A row the open transaction inserted, whose table a Get through a Record variable read after a Get of another table sent a higher row version, has a SystemRowVersion greater than 0 in the buffer that variable's own Get loaded. A read of the field in that buffer gives a value known only to be greater than 0: a comparison with 0 answers, every other use refuses. Every other shape keeps the refusal: a re-read by a Find, Next or RecordRef, a copy of the buffer, the variable's own Insert or Modify buffer, and a row sent by a Commit, a write or another read, or after a higher row version a write or another read sent (al-vm records/restamped.rs `restamped_value`, `re_read_by_get`; records/stale.rs `note_read_for_row_versions`).",
    tests: "al-runner issue 8993. Corpus PR 1509 (aldevtools/BusinessCentral.AL.Language.Tests, merged, not pinned yet), session-system/row-version-writes/RowVersionWrites.al:392 LastUsedRowVersion_InsertTwoTables, observed on a local Business Central: Row.Insert; Other.Insert; Other.Get; Row.Get, then `row/other` greater (:420), with Other's version above the LastUsedRowVersion read before the Inserts, which is greater than 0 (metadata/platform-tables/PlTDatabase.al:229).",
    probe: "session-system/row-version-writes/RowVersionWrites.al of corpus PR 1509 (merged), not yet pinned. Only relations are measured, never a number. Still to measure: the row sent by a Commit, a write or a read other than Get; the higher row version sent by a write or a read other than Get; the row re-read by a Find, Next or RecordRef read, or a copy of the buffer; what the variable's own Insert buffer holds after the send.",
    awaiting: Awaiting {
        issues: &[8993],
        files: &["tests/al-language/session-system/row-version-writes/RowVersionWrites.al"],
        since: "2026-11-26",
    },
}
