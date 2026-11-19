TransferFieldsSkipsRemovedTargetField => {
    id: "transferfields-skips-removed-target-field",
    assumed: "TransferFields skips a field of the target table that is declared ObsoleteState = Removed: it compares no type with the source's field of that number, raises no error and takes no value. Pinned and not assumed: to RecordRef a removed field is not a field of its table (recordref-fieldref/PnD12_RemovedFieldRecordRef.al: FieldCount, FieldExist, FieldIndex). A removed field of the source against a live field of the target is refused, not assumed.",
    tests: "No Microsoft test: the Base Application source of the artifact cache declares no removed field. al-runner issue 8494: the reporter's repro passes on Business Central 29.0 W1 OnPrem (build 29.0.54011.55816): Target.TransferFields(Source) into a table whose field 2 is a removed Code[5] while the source's field 2 is a Boolean, and Sales Shipment Line.InitFromSalesLine with a removed Code[5] extension field and a Boolean extension field of one app under one number; the control, the same field not removed, raises the type mismatch.",
    probe: "TransferFields into a table with a removed field of another type and of the same type (a value too long for it included), on two tables of one app and through table extensions of one app, with InitPrimaryKeyFields false and SkipFieldsNotMatchingType true; and from a table with a removed field into a live field of that number: does the target keep its value?",
    awaiting: Awaiting {
        issues: &[8494],
        files: &["tests/al-language/record/TransferFieldsRemovedField.al"],
        since: "2026-11-19",
    },
}
