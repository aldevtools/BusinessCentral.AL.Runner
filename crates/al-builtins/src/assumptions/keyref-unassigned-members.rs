KeyRefUnassignedMembers => {
    id: "keyref-unassigned-members",
    assumed: "On a KeyRef nothing assigned, FieldCount, Active, FieldIndex(1) and Record with the result used raise \"Microsoft.Dynamics.Nav.Runtime.NavKeyRef variable not initialized.\" (NotInitialized). On a KeyRef that held a key, after Clear, only Active with the result used raises it, and only on the cleared variable itself, a declared local variable of a codeunit procedure (corpus PR 1411, merged, not yet pinned; FieldCount there was observed, not asserted). A copy by assignment or through a Variant, a by-value or var parameter, a global and an array element keep their answer before this assumption (a refusal). FieldCount, FieldIndex and Record after Clear, any member of a KeyRef cleared while it held nothing, other arguments of FieldIndex and the statement forms keep their answer before this assumption (a refusal).",
    tests: "Outside report: al-runner issue 8001 (KeyRef). Measured on every supported major by corpus PR 1408 (merged, not yet pinned).",
    probe: "Corpus PR 1408: tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al (codeunit 66991, KeyRef_* tests). After Clear: corpus PR 1411 (merged, not yet pinned), streams/instream-outstream/UhUntouchedStreamsAfter.al, KeyRef_AfterClear_Members.",
    awaiting: Awaiting {
        issues: &[8001],
        files: &[
            "tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al",
            "tests/al-language/streams/instream-outstream/UhUntouchedStreamsAfter.al",
        ],
        since: "2026-11-15",
    },
}
