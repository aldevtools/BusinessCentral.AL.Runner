RecordRefGetTableSharedSameTable => {
    id: "record-ref-get-table-shared-same-table",
    assumed: "GetTable from a typed persistent record into an open RecordRef of the same table updates the buffer it shares with a RecordRef it was assigned from or to, or with the RecordRef a by-value event argument of a subscriber was copied from. A RecordRef assigned from a Variant, a temporary record or RecordRef, and a source passed through a Variant keep their own buffer. GetTable with another table is known to differ on native BC (it also reaches the original) and is still open.",
    tests: "AL Runner issue 7885 (reporter's repro GetTableInByValueSubscriberChangesPublisherRow and GetTableUpdatesAnAssignedAliasOfTheSameTable, measured on all three supported BC majors; the issue records the exact builds).",
    probe: "Corpus pull request 1357.",
    awaiting: Awaiting {
        issues: &[7885],
        files: &["tests/al-language/recordref-fieldref/recordref/TestRecordRefGetTableAlias.al"],
        since: "2026-11-12",
    },
}
