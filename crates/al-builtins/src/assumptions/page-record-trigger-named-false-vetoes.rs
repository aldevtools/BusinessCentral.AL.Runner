PageRecordTriggerNamedFalseVetoes => {
    id: "page-record-trigger-named-false-vetoes",
    assumed: "A page's own OnInsertRecord or OnModifyRecord with a named Boolean return value that a top-level statement assigns the constant false, and no exit, vetoes: the insert by the SetValue of a field outside the key raises `Validation error for Field: ,  Message = 'The view is filtered, and the entry is outside the filter. Some actions may not work.'` (TestValidation) and stores nothing, and every later call on the page is refused; the modify at Next or Close is dropped without an error (al-vm pages/record_trigger_answer.rs). Only the probe's shape: a Card over a stored table, top-level, without parts, opened by the test with OpenNew or OpenEdit, no page extension that declares the trigger and no subscriber to its event. The insert at another step, and every other shape of a named return value that ends false, are refused.",
    tests: "No Microsoft test.",
    probe: "Open corpus PR 1367 (pages/triggers/RecordTriggerReturnValue.al, group G6): :350 False_Insert_OpenNewSetValueClose, :373 _Next, :397 False_Modify_SetValueNext, :422 _Close pass on every major of its CI run on 32b29320.",
    awaiting: Awaiting {
        issues: &[7628],
        files: &["tests/al-language/pages/triggers/RecordTriggerReturnValue.al"],
        since: "2026-11-13",
    },
}
