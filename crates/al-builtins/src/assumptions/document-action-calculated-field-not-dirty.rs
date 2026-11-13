DocumentActionCalculatedFieldNotDirty => {
    id: "document-action-calculated-field-not-dirty",
    assumed: "On a clean top-level Document page without parts, opened in view mode by the test's OpenView, whose only test calls are GoToRecord and then the action's Invoke with no store write between them, an action that only changes FlowFields of Rec with CalcFields and committed its write through a separate record buffer causes no implicit page save. The page and its extensions declare no OnModifyRecord and nothing subscribes to it.",
    tests: "AL Runner issue 7882 (reporter's repro CalculatedFieldDoesNotSaveStaleDocument, measured on all three supported BC majors; the issue records the exact builds).",
    probe: "Corpus pull request 1346.",
    awaiting: Awaiting {
        issues: &[7882],
        files: &["tests/al-language/pages/currpage/TestDocumentFlowFieldSave.al"],
        since: "2026-11-12",
    },
}
