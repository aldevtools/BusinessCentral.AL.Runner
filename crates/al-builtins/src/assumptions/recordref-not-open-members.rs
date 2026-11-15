RecordRefNotOpenMembers => {
    id: "recordref-not-open-members",
    assumed: "On a RecordRef that is not open, these members raise \"The record is not open.\" (RecordNotOpened) in the call form and with the arguments the probe used. Never opened and opened then closed: CopyLinks as a statement whose argument is a Record (the Record overload) or an open RecordRef (the RecordRef overload), not temporary, in the session's company, with no filter and no mark, standing on no row and with nothing written to its buffer (not a Variant); FindSet, ReadPermission, WritePermission, FieldCount and FindFirst with no argument and the result used (FieldCount of a RecordRef never opened is pinned: RfTOpen.al:191, :630). Never opened only: FindLast, GetFilters and Next with no argument and the result used; KeyIndex(1) with the result used; SetRecFilter with no argument and SetLoadFields(1) as statements. An Open that raised and a Get(RecordId) that left a ref never opened not open count as a close. A RecordRef that was open, after Clear, and a RecordRef never opened, after an Open(0) that raised nothing: only FindLast, GetFilters and Next with no argument and the result used, KeyIndex(1) with the result used, SetRecFilter with no argument and SetLoadFields(1) as statements (corpus PR 1411; FieldCount after Open(0) is pinned: RfTOpen.al:630). Every other member, form, argument and state keeps its answer before this assumption (a refusal), among them Clear of a RecordRef that was not open, and an Open or a Get that left a cleared or Open(0) RecordRef not open.",
    tests: "Outside reports: al-runner issues 7760, 7764 (RR.ReadPermission on a RecordRef never opened) and 8084. Measured on every supported major by corpus PR 1408 (merged, not yet pinned).",
    probe: "Corpus PR 1408: tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al (codeunit 66991, Untouched_* and Closed_* tests). After Clear and after Open(0), and the shapes this entry still refuses (after a failed Open or a missed Get, other CopyLinks sources): corpus PR 1411 (merged, not yet pinned), codeunit 66996 in streams/instream-outstream/UhUntouchedStreamsAfter.al.",
    awaiting: Awaiting {
        issues: &[7760, 7764, 8084],
        files: &[
            "tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al",
            "tests/al-language/streams/instream-outstream/UhUntouchedStreamsAfter.al",
        ],
        since: "2026-11-15",
    },
}
