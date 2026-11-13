BlobAssignCountsAsWrite => {
    id: "blob-assign-counts-as-write",
    assumed: "`Target.Field := Source.Blob` counts as an unsaved write to the target's Blob, as a write through an OutStream does (streams/TestBlobCalcFieldsUncommittedWrite.al): `CalcFields` on the target with no Modify in between keeps the assigned value and does not fetch the stored one. Covered, the reporter's shape only: the source is a temporary record's Blob that a stream wrote and its Insert stored, with no stream created on it since; the target is a stored record's loaded Blob without content and without an unsaved write, and the stored row's Blob is empty. Every other source and target (a stored source read with CalcFields, a source with an unsaved write, a target with content, a target read without CalcFields, a temporary target) is copied as before this assumption. Not told apart: a temporary source whose content came by an assignment before its Insert.",
    tests: "Outside report: al-runner issue 7961, the reporter's repro (AssignFromTempThenCalcFields: before=Yes after=Yes on real BC, measured by the reporter through Microsoft's test runner).",
    probe: "Not written yet. Spec: tests/al-language/streams/BlobAssignThenCalcFields.al. `Row.Content := Source.Content`, then `Row.CalcFields(Content)` with no Modify, HasValue and text before and after; Row stored (inserted empty, inserted with other content) and temporary; Source temporary after Insert, stored and read with CalcFields, written in memory and never saved; then Modify and the stored row; then another variable modifies the row before CalcFields.",
    awaiting: Awaiting {
        issues: &[7961],
        files: &["tests/al-language/streams/BlobAssignThenCalcFields.al"],
        since: "2026-11-13",
    },
}
