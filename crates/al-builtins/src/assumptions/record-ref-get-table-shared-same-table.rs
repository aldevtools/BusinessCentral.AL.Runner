RecordRefGetTableSharedSameTable => {
    id: "record-ref-get-table-shared-same-table",
    assumed: "GetTable from a typed persistent record into an open RecordRef of the same table updates the buffer it shares with the RecordRef it was assigned from, or, for a by-value RecordRef event argument of a subscriber, with the RecordRef it was copied from. GetTable on the RecordRef that was assigned to another keeps its own buffer. A RecordRef assigned from a Variant (also a `var` Variant parameter bound to a RecordRef), a temporary record or RecordRef, and a source passed through a Variant keep their own buffer. GetTable with another table is known to differ on native BC (it also reaches the original) and is still open.",
    tests: "AL Runner issue 7885 (reporter's repro GetTableInByValueSubscriberChangesPublisherRow and GetTableUpdatesAnAssignedAliasOfTheSameTable, measured on all three supported BC majors; the issue records the exact builds).",
    probe: "Corpus pull request 1357.",
    awaiting: Awaiting {
        issues: &[7885],
        files: &["tests/al-language/recordref-fieldref/recordref/TestRecordRefGetTableAlias.al"],
        since: "2026-11-12",
    },
}
