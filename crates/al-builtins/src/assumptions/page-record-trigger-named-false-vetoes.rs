PageRecordTriggerNamedFalseVetoes => {
    id: "page-record-trigger-named-false-vetoes",
    assumed: "A page's own OnInsertRecord or OnModifyRecord with a named Boolean return value that ends false vetoes: the insert by the SetValue of a field outside the key raises `Validation error for Field: ,  Message = 'The view is filtered, and the entry is outside the filter. Some actions may not work.'` (TestValidation) and stores nothing, and every later call on the page is refused; the modify at Next or Close is dropped without an error (al-vm pages/record_trigger_answer.rs). Only the probe's shape: a Card over a stored table, top-level, without parts, no other routine on the page, its three record triggers each one call of a procedure of a codeunit page variable with text of constants and fields of Rec as argument (a procedure that only appends text to one Text variable of its codeunit), then `Ok := false;` with `Ok` the named return value, the table with OnInsert and OnModify only, no page extension and no subscriber; opened by the test with OpenNew (insert by the SetValue of a field outside the key after the SetValue of the key) or OpenEdit (GoToKey, SetValue of a field outside the key, modify at Close or at a Next that moves). The insert at another step, and every other shape where an assignment in the body leaves the named return value false, are refused; a value set false only through a `var` parameter is not seen and keeps the earlier answer (saved).",
    tests: "No Microsoft test.",
    probe: "Open corpus PR 1367 (pages/triggers/RecordTriggerReturnValue.al, group G6): :350 False_Insert_OpenNewSetValueClose, :373 _Next, :397 False_Modify_SetValueNext, :422 _Close pass on every major of its CI run on 32b29320.",
    awaiting: Awaiting {
        issues: &[7628],
        files: &["tests/al-language/pages/triggers/RecordTriggerReturnValue.al"],
        since: "2026-11-13",
    },
}
