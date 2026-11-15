RecordRefGetTableVariantNotRecord => {
    id: "recordref-gettable-variant-not-record",
    assumed: "RecordRef.GetTable(V) as a statement on a RecordRef never opened (not one after Clear, Open(0) or Close), where the Variant V holds the Text '' or 'abc', raises \"Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to Microsoft.Dynamics.Nav.Runtime.INavRecordHandle.\" (Conversion). The Integer 5, the Text 'ALT Universal' and an unassigned Variant are pinned (GetTableVariantKinds.al). Other texts, other values and types, an open RecordRef and one closed, cleared or after Open(0) keep their answer before this assumption (a refusal).",
    tests: "Outside report: al-runner issue 8084 (RecordRef.GetTable of an empty Text). Measured on every supported major by corpus PR 1408 (merged, not yet pinned).",
    probe: "Corpus PR 1408: tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al (codeunit 66991, GetTable_Variant* tests).",
    awaiting: Awaiting {
        issues: &[8084],
        files: &["tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al"],
        since: "2026-11-15",
    },
}
