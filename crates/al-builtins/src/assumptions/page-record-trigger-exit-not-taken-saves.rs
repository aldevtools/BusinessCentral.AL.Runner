PageRecordTriggerExitNotTakenSaves => {
    id: "page-record-trigger-exit-not-taken-saves",
    assumed: "A page's own OnInsertRecord or OnModifyRecord with an unnamed Boolean return that has an `exit(value)` and ends without running one does not veto: the insert or the modify is saved (al-vm pages/record_trigger_answer.rs). Only two shapes. The probe's: a Card over a stored table, top-level, without parts, no other routine on the page, its three record triggers each one call of a procedure of a codeunit page variable with text of constants and fields of Rec as argument, then `if <page Boolean variable> then exit(false);`, the table with OnInsert and OnModify only, no page extension and no subscriber; opened by the test with OpenNew (insert by the SetValue of a field outside the key after the SetValue of the key; modify at the Next or Close after them) or OpenEdit (GoToKey, SetValue of a field outside the key, modify at Next or Close). The corpus page testpage/CcFixtures.al:244: a Card whose only routine is OnModifyRecord, one call of a procedure of a codeunit local variable, then one `case` without `else`, the table without code; opened with OpenEdit, then GoToKey, SetValue of a field outside the key, Close. Every other shape where such a trigger ends without its exit is refused.",
    tests: "No Microsoft test.",
    probe: "Open corpus PR 1367 (pages/triggers/RecordTriggerReturnValue.al, group G6): :252 Cond_Insert_OpenNewSetValueClose, :276 _Next, :301 Cond_Modify_SetValueNext, :326 _Close pass on every major of its CI run on 32b29320.",
    awaiting: Awaiting {
        issues: &[7628],
        files: &["tests/al-language/pages/triggers/RecordTriggerReturnValue.al"],
        since: "2026-11-13",
    },
}
