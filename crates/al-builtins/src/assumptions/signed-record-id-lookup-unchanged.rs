SignedRecordIdLookupUnchanged => {
    id: "signed-record-id-lookup-unchanged",
    assumed: "A RecordId FlowField with `CalcFormula = -lookup(...)` gives the stored RecordId unchanged. Pinned: CalcFields gives a RecordId equal to the stored one (record/flowfields/TestSignedRecordIdLookup.al:25, corpus pull request 1199). A sign on min or max, or on a lookup of another type, is not pinned and stays refused.",
    tests: "No Microsoft test: al-runner issue 7266 (the reporter's real-BC run of CalcFormula_SignedRecordIdLookup_ReturnsStoredTarget on the three supported majors, corpus pull request 1199).",
    probe: "record/flowfields/TestSignedRecordIdLookup.al once merged; also wanted: a signed lookup of an Integer, Decimal, Code and Date field, and signed min and max.",
    awaiting: Awaiting {
        issues: &[7266],
        files: &["tests/al-language/record/flowfields/TestSignedRecordIdLookup.al"],
        since: "2026-11-08",
    },
}
