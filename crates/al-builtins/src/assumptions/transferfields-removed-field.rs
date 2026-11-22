TransferFieldsRemovedField => {
    id: "transferfields-removed-field",
    assumed: "TransferFields with a field declared ObsoleteState = Removed, source and target fields of one app, for the cells corpus PR 1486 measured (each with its InitPrimaryKeyFields): a removed target field raises nothing and takes no value (Boolean into Code[5], any InitPrimaryKeyFields, also two table extension fields; Integer into Integer with SkipFieldsNotMatchingType false, any InitPrimaryKeyFields; Text into Integer and Code[20] into Code[5], a value too long included, InitPrimaryKeyFields true); a removed source field leaves a live target field of that number as it was (Boolean into Integer with InitPrimaryKeyFields true, or false with SkipFieldsNotMatchingType true; Integer into Integer with InitPrimaryKeyFields true); both removed, Boolean into Code[5] with InitPrimaryKeyFields true, raises nothing. Every other cell (other type pairs, Code lengths, InitPrimaryKeyFields values, fields of two apps) keeps main's skip or its copy into a removed target field, and is refused where main would copy a removed source field or raise an error.",
    tests: "No Microsoft test: the Base Application source of the artifact cache declares no removed field. Corpus PR 1486 (merged, not yet in the pinned corpus): record/TransferFieldsRemovedField.al:428-891, the same values on the three pinned majors (Windows, W1). al-runner issue 8494: the reporter's repro passes on Business Central 29.0 W1 OnPrem.",
    probe: "Written: corpus PR 1486, record/TransferFieldsRemovedField.al. Not measured: removed fields of two apps, other type pairs, RecordRef and FieldRef based copies.",
    awaiting: Awaiting {
        issues: &[8494],
        files: &["tests/al-language/record/TransferFieldsRemovedField.al"],
        since: "2026-11-21",
    },
}
