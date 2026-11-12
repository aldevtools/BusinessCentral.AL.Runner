FieldrefValueBlobCopy => {
    id: "fieldref-value-blob-copy",
    assumed: "`FieldRef.Value := X`, where X is a loaded Blob that another holder still has (a record field, a Variant), on a FieldRef of a temporary record gives the field a Blob of its own, as `Target.Field := Source.Blob` does (record/lt-blob-field-assign/BlobFieldAssignCopy.al:73-127): a later write to the source, or a Clear of the record's field after an Insert stored it, does not change the field or the stored row. A FieldRef of a stored record, and an unloaded source Blob, keep the earlier result (the field shares the source's buffer).",
    tests: "System Application Test, codeunit 135032 Temp Blob List Test: RemoveAtVerifyRemainingList, RemoveRange, RemoveRangeElementCountOutOfRange, RemoveElementsFromHead (Temp Blob List Impl. CopyBlob: Clear, RecordRef.GetTable, Temp Blob ToRecordRef, RecordRef.SetTable, Insert), pass on BC. Outside report: al-runner issue 7935, the reporter's repro.",
    probe: "Not written yet. Wanted: FieldRef.Value := Src.Blob into a temporary and into a stored record (RecordRef.GetTable and RecordRef.Open), then a write to Src, a Clear of Src, a Clear of the target field after Insert, and Insert/Get of a second row; the source loaded, read with Get without CalcFields, and held in a Variant; read the target field, the RecordRef field and the stored row. Planned file: tests/al-language/recordref-fieldref/fieldref/FieldRefValueBlobCopy.al.",
    awaiting: Awaiting {
        issues: &[7935],
        files: &["tests/al-language/recordref-fieldref/fieldref/FieldRefValueBlobCopy.al"],
        since: "2026-11-12",
    },
}
