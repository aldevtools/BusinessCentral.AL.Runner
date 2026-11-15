RecordRefNotOpenMembers => {
    id: "recordref-not-open-members",
    assumed: "On a RecordRef that is not open, these members raise \"The record is not open.\" (RecordNotOpened) in the call form and with the arguments the probe used. Never opened and opened then closed: CopyLinks as a statement with a non-temporary open RecordRef or a non-temporary Record as the source; FindSet, ReadPermission, WritePermission, FieldCount and FindFirst with no argument and the result used (FieldCount of a RecordRef never opened is pinned: RfTOpen.al:191). Never opened only (also after Clear): FindLast, GetFilters and Next with no argument and the result used; KeyIndex(1) with the result used; SetRecFilter with no argument and SetLoadFields(1) as statements. Every other member, form, argument and state keeps its answer before this assumption (a refusal).",
    tests: "Outside reports: al-runner issues 7760, 7764 (RR.ReadPermission on a RecordRef never opened) and 8084. Measured on every supported major by corpus PR 1408 (merged, not yet pinned).",
    probe: "Corpus PR 1408: tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al (codeunit 66991, Untouched_* and Closed_* tests).",
    awaiting: Awaiting {
        issues: &[7760, 7764, 8084],
        files: &["tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al"],
        since: "2026-11-15",
    },
}
