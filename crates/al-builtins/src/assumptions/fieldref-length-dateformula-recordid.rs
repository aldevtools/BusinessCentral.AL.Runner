FieldRefLengthDateFormulaRecordId => {
    id: "fieldref-length-dateformula-recordid",
    assumed: "FieldRef.Length of a DateFormula field is 32 and of a RecordId field 448 (real BC 29 values that corpus writer Y66 measured, not on corpus master yet).",
    tests: "No Microsoft test: al-runner issue 6663, a field-transfer helper that compares Type and Length of every field pair.",
    probe: "FieldRef.Length of a DateFormula and a RecordId field, a temporary record included (written: Y66FieldRefLengthTypes.al:61-141).",
    awaiting: Awaiting {
        issues: &[6663],
        files: &["tests/al-language/fieldref/lt-y66-fieldref-length-types/Y66FieldRefLengthTypes.al"],
        since: "2026-11-01",
    },
}
