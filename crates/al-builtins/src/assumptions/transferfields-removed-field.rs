TransferFieldsRemovedField => {
    id: "transferfields-removed-field",
    assumed: "TransferFields with a field declared ObsoleteState = Removed, source and target fields of one app, for the cells corpus PR 1486 measured: a removed target field raises nothing and takes no value (Boolean into Code[5], also two table extension fields; Integer into Integer with SkipFieldsNotMatchingType false; Text into Integer; Code[20] into Code[5], a value too long included); a removed source field leaves a live target field of that number as it was (Boolean into Integer, Integer into Integer); both removed, Boolean into Code[5], raises nothing. Any argument values otherwise. Every other type pair, and fields of two apps, keep main's skip or its copy into a removed target field, and are refused where main would copy a removed source field or raise an error.",
    tests: "No Microsoft test: the Base Application source of the artifact cache declares no removed field. Corpus PR 1486 (merged, not yet in the pinned corpus): record/TransferFieldsRemovedField.al:428-891, the same values on BC 27.5, 28.5 and 29.0 (Windows, W1). al-runner issue 8494: the reporter's repro passes on BC 29.0 W1 OnPrem.",
    probe: "Written: corpus PR 1486, record/TransferFieldsRemovedField.al. Not measured: removed fields of two apps, other type pairs, RecordRef and FieldRef based copies.",
    awaiting: Awaiting {
        issues: &[8494],
        files: &["tests/al-language/record/TransferFieldsRemovedField.al"],
        since: "2026-11-21",
    },
}
