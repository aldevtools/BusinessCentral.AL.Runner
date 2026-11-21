TransferFieldsSkipsRemovedTargetField => {
    id: "transferfields-skips-removed-target-field",
    assumed: "TransferFields skips a field of the target table that is declared ObsoleteState = Removed, in the shape of the reporter's repro: the target's field a Code field, the source's field of that number a live Boolean field, both fields of one app, InitPrimaryKeyFields true and SkipFieldsNotMatchingType false. It compares no type, raises no error and takes no value. Pinned and not assumed: to RecordRef a removed field is not a field of its table (recordref-fieldref/PnD12_RemovedFieldRecordRef.al:90-93 FieldCount and FieldExist, :112 Field(n), :132 FieldIndex). Not assumed either: into a removed field of the very same type the skip is unflagged (the copy could not fail, and no reader sees the field). Every other shape past a removed target field is refused, and so is a removed field of the source where the field would be copied or raise an error.",
    tests: "No Microsoft test: the Base Application source of the artifact cache declares no removed field. al-runner issue 8494: the reporter's repro passes on Business Central 29.0 W1 OnPrem (build 29.0.54011.55816): Target.TransferFields(Source) into a table whose field 2 is a removed Code[5] while the source's field 2 is a Boolean, and Sales Shipment Line.InitFromSalesLine with a removed Code[5] extension field and a Boolean extension field of one app under one number; the control, the same field not removed, raises the type mismatch.",
    probe: "TransferFields into a table with a removed field in the shapes the repro did not measure: other type pairs (an Integer field from a Text field), a Code value too long for the removed field, InitPrimaryKeyFields false, SkipFieldsNotMatchingType true, a source field that is removed too, fields of two apps; and from a table with a removed field into a live field of that number where the field would be copied or raise an error: does the target keep its value?",
    awaiting: Awaiting {
        issues: &[8494],
        files: &["tests/al-language/record/TransferFieldsRemovedField.al"],
        since: "2026-11-19",
    },
}
