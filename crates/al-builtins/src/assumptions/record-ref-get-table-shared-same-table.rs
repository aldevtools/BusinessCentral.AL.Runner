RecordRefGetTableSharedSameTable => {
    id: "record-ref-get-table-shared-same-table",
    assumed: "GetTable from a typed persistent record into an open RecordRef of the same table updates its shared record buffer.",
    tests: "Local neutral RecordRefGetTable.al probe with a by-value integration event subscriber.",
    probe: "Add same-table GetTable through RecordRef aliases to the pinned corpus.",
}
