PageRecordTriggerExitNotTakenSaves => {
    id: "page-record-trigger-exit-not-taken-saves",
    assumed: "A page's own OnInsertRecord or OnModifyRecord with an unnamed Boolean return whose every exit is `exit(false)` and that ends without running one does not veto: the insert or the modify is saved (al-vm pages/record_trigger_answer.rs). Only the probe's shape: a Card over a stored table, top-level, without parts, opened by the test with OpenNew (insert by the SetValue of a field outside the key, or at Close or Next) or OpenEdit (modify at Next or Close), no page extension that declares the trigger and no subscriber to its event. Every other shape where such a trigger ends without its exit is refused.",
    tests: "No Microsoft test.",
    probe: "Open corpus PR 1367 (pages/triggers/RecordTriggerReturnValue.al, group G6): :252 Cond_Insert_OpenNewSetValueClose, :276 _Next, :301 Cond_Modify_SetValueNext, :326 _Close pass on every major of its CI run on 32b29320.",
    awaiting: Awaiting {
        issues: &[7628],
        files: &["tests/al-language/pages/triggers/RecordTriggerReturnValue.al"],
        since: "2026-11-13",
    },
}
