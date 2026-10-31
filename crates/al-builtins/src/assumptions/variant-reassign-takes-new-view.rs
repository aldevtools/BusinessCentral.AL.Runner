VariantReassignTakesNewView => {
    id: "variant-reassign-takes-new-view",
    assumed: "A Variant that holds a record and is given another (not temporary) record of the same table holds the new record whole, as a Variant that held nothing would: RecordRef.GetTable of it takes the new record's filters and a record assigned from it takes the new record's fields.",
    tests: "No Microsoft test: al-runner issue 6503, the reporter's run on real BC, newest major (first{count=1 filters=[No: A]} second{count=0 filters=[No: NONE]} back{count=1 filters=[]}; SetFilter, a reused record and GetTable in another codeunit also count 0).",
    probe: "Corpus writer Y23 (corpus branch al-runtime-rs-staging-y23, Y23VariantRecordView.al): the reporter's repro and its variants (SetFilter, a reused record, another codeunit by var and by value, a temporary record on either side, which still takes only the fields).",
    awaiting: Awaiting {
        issues: &[6503],
        files: &["tests/al-language/recordref/lt-y23-variant-view/Y23VariantRecordView.al"],
        since: "2026-10-31",
    },
}
