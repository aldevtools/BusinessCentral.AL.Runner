KeyRefUnassignedMembers => {
    id: "keyref-unassigned-members",
    assumed: "On a KeyRef nothing assigned (also after Clear), FieldCount, Active, FieldIndex(1) and Record with the result used raise \"Microsoft.Dynamics.Nav.Runtime.NavKeyRef variable not initialized.\" (NotInitialized). Other arguments of FieldIndex and the statement forms keep their answer before this assumption (a refusal).",
    tests: "Outside report: al-runner issue 8001 (KeyRef). Measured on every supported major by corpus PR 1408 (merged, not yet pinned).",
    probe: "Corpus PR 1408: tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al (codeunit 66991, KeyRef_* tests).",
    awaiting: Awaiting {
        issues: &[8001],
        files: &["tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al"],
        since: "2026-11-15",
    },
}
