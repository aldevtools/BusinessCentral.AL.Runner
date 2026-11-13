BlobAssignCountsAsWrite => {
    id: "blob-assign-counts-as-write",
    assumed: "`Target.Field := Source.Blob` counts as an unsaved write to the target's Blob, as a write through an OutStream does (streams/TestBlobCalcFieldsUncommittedWrite.al): `CalcFields` on the target with no Modify in between keeps the assigned value and does not fetch the stored one. Covered: a loaded source Blob with content that holds no unsaved write itself (the Blob of a temporary record after its Insert), assigned to a field whose Blob holds no unsaved write. A source that holds an unsaved write, and a target that holds one, are copied as before this assumption.",
    tests: "Outside report: al-runner issue 7961, the reporter's repro (AssignFromTempThenCalcFields: before=Yes after=Yes on real BC, measured by the reporter through Microsoft's test runner).",
    probe: "Not written yet. Spec: tests/al-language/streams/BlobAssignThenCalcFields.al. `Row.Content := Source.Content`, then `Row.CalcFields(Content)` with no Modify, HasValue and text before and after; Row stored (inserted empty, inserted with other content) and temporary; Source temporary after Insert, stored and read with CalcFields, written in memory and never saved; then Modify and the stored row; then another variable modifies the row before CalcFields.",
    awaiting: Awaiting {
        issues: &[7961],
        files: &["tests/al-language/streams/BlobAssignThenCalcFields.al"],
        since: "2026-11-13",
    },
}
