RecordRefNotOpenMembers => {
    id: "recordref-not-open-members",
    assumed: "On a RecordRef that is not open, these members raise \"The record is not open.\" (RecordNotOpened) in the call form and with the arguments the probe used. Never opened and opened then closed: CopyLinks as a statement whose argument is a Record (the Record overload) or an open RecordRef (the RecordRef overload), not temporary, in the session's company, with no filter and no mark, standing on no row and with nothing written to its buffer (not a Variant); FindSet, ReadPermission, WritePermission, FieldCount and FindFirst with no argument and the result used (FieldCount of a RecordRef never opened is pinned: RfTOpen.al:191, :630). Never opened only: FindLast, GetFilters and Next with no argument and the result used; KeyIndex(1) with the result used; SetRecFilter with no argument and SetLoadFields(1) as statements. An Open that raised and a Get(RecordId) that left the ref not open count as a close. Not checked: a RecordRef after Clear and after an Open(0) that raised nothing counts as never opened (neither is measured for these members). Every other member, form, argument and state keeps its answer before this assumption (a refusal).",
    tests: "Outside reports: al-runner issues 7760, 7764 (RR.ReadPermission on a RecordRef never opened) and 8084. Measured on every supported major by corpus PR 1408 (merged, not yet pinned).",
    probe: "Corpus PR 1408: tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al (codeunit 66991, Untouched_* and Closed_* tests). The shapes not checked (after Clear, after Open(0)) and the refused ones (after a failed Open or a missed Get, other CopyLinks sources): observe-form draft, codeunit 66996 in streams/instream-outstream/UhUntouchedStreamsAfter.al, not yet opened.",
    awaiting: Awaiting {
        issues: &[7760, 7764, 8084],
        files: &["tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al"],
        since: "2026-11-15",
    },
}
