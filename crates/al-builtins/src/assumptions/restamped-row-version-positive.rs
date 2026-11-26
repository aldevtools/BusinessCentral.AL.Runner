RestampedRowVersionPositive => {
    id: "restamped-row-version-positive",
    assumed: "A row the open transaction inserted, sent after a higher row version was sent (by a read or write of its table, or a Commit), and read again from the store (a Get, a Find or Next, a RecordRef read, or a copy of a buffer such a read filled) has a SystemRowVersion greater than 0, and greater than every row version sent before it. A Find, Next or RecordRef read gives the stored row's version as a Get does. A read of the field gives a value known only to be greater than 0: a comparison with 0 answers, every other use refuses. The variable's own Insert or Modify buffer keeps the refusal (al-vm records/restamped.rs `restamped_value`, `re_read`).",
    tests: "al-runner issue 8993. Corpus PR 1509 (aldevtools/BusinessCentral.AL.Language.Tests, merged, not pinned yet), session-system/row-version-writes/RowVersionWrites.al:392 LastUsedRowVersion_InsertTwoTables, observed on a local Business Central: Row.Insert; Other.Insert; Other.Get; Row.Get, then `row/other` greater (:420), with Other's version above the LastUsedRowVersion read before the Inserts, which is greater than 0 (metadata/platform-tables/PlTDatabase.al:229).",
    probe: "session-system/row-version-writes/RowVersionWrites.al of corpus PR 1509 (merged), not yet pinned. Only relations are measured, never a number. Still to measure: the row sent by a Commit or by a write of its table, re-read by a Find, Next or RecordRef read, and what the variable's own Insert buffer holds after the send.",
    awaiting: Awaiting {
        issues: &[8993],
        files: &["tests/al-language/session-system/row-version-writes/RowVersionWrites.al"],
        since: "2026-11-26",
    },
}
