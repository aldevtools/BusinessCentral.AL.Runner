VariantKeepsRecordrefAfterClose => {
    id: "variant-keeps-recordref-after-close",
    assumed: "A Variant assigned a RecordRef (`V := RecRef`) holds a RecordRef of its own on the same record buffer: when the original then leaves that buffer (RecordRef.Close, or Open or GetTable again), the Variant still holds the open RecordRef (Format(V) is not empty, V.IsRecordRef, `R := V` gives the table, the row and its field values), also after the Variant was passed by value; and a Close of the RecordRef taken out of the Variant (`R := V; R.Close()`) leaves the original open. The corpus pins only the shared buffer while both are open (types/variant/CxVariants.al:194-198, record/ProbePropertyAssignmentForms.al:833-835), which is not marked. A Close after `RecRef2 := RecRef` is not pinned either and still closes both (not changed here).",
    tests: "No Microsoft test. An outside reporter's test passes on real BC and needs it (al-runner issue 6924, Test Data Check MessageResultsToRecords: `CheckRecord := CheckRecRef; CheckRecRef.Close();`, then Format(CheckRecord) <> '' and Data Type Management.GetRecordRef(CheckRecord, R) gives table 36).",
    probe: "tests/al-language/recordref-fieldref/recordref/RecordRefInVariantAfterClose.al (corpus pull request 1299): after `V := RecRef; RecRef.Close()` the Format of the Variant, IsRecordRef, and Number, IsEmpty, Find and a field value of the RecordRef taken back out (plain `R := V` and Data Type Management.GetRecordRef); a Close of the taken-out RecordRef and what the Variant and the original hold then; the Variant passed by value and by var; `RecRef2 := RecRef` then Close of either; `V := Record` for contrast; Format of a Variant holding a Record or a RecordRef with no Close.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/recordref-fieldref/recordref/RecordRefInVariantAfterClose.al"],
        since: "2026-11-10",
    },
}
