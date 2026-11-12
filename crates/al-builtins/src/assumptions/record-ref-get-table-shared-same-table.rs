RecordRefGetTableSharedSameTable => {
    id: "record-ref-get-table-shared-same-table",
    assumed: "GetTable from a typed persistent record into an open RecordRef of the same table updates its shared record buffer.",
    tests: "AL Runner issue 7885, measured on BC 29.0. Another-table GetTable also reaches the original on native BC and remains outside this assumption.",
    probe: "Corpus pull request 1357.",
}
