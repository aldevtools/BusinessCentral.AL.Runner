ListReopenAfterModeSwitch => {
    id: "list-reopen-after-mode-switch",
    assumed: "On the probe's list: a page that is not a part, whose page, source table and called procedure read line for line as the probe's page 75641, table 75640 and Add of log codeunit 75642 (properties, controls, trigger bodies, fields and keys; under any object names; the table's lines leave out the platform's own fields as each major has them, and the page shows none of them), with no extension and no subscriber of the page, the table or the log codeunit, Rec in the plain view of its primary key (no filter, ascending, no marks) (al-vm pages/list_view_switch.rs `reopen_probe_list`): after the built-in View and then Edit switched it back to edit, or after Edit switched the page opened for view to edit, and TestPage.Close() then closed it cleanly, the next OpenEdit in the test method the page opened in (after View and Edit also OpenView and OpenNew) opens a fresh page, as a first open does; OpenEdit while the page after View and Edit is still open raises 'The TestPage is already open.' and leaves the page as it was (al-vm pages/list_view_switch.rs `reopen_after_switch_measured`). Every other open after these switches keeps the refusal.",
    tests: "No Microsoft test. Corpus PR 1467 (merged, not yet pinned), measured on a local BC of each of the three supported majors, the same on each.",
    probe: "Corpus PR 1467, tests/al-language/testpage/lifecycle/ReopenAfterListModeSwitch.al: ViewEdit_Close_OpenEdit_Reads, _Walk, _SwitchAgain, ViewEdit_SetValue_Close_OpenEdit_ShowsValue, ViewEdit_Close_OpenView_Reads, _WalkThenEdit, ViewEdit_Close_OpenNew_Reads, OpenView_Edit_Close_OpenEdit_Reads, ViewEdit_OpenEditWithoutClose. Not measured: OpenView or OpenNew after Edit on a page opened for view, OpenView or OpenNew on the open page, another close than TestPage.Close(), a reopen in a later test method (a global TestPage), a filter or sort, a FlowField on the table, other pages.",
    awaiting: Awaiting {
        issues: &[8501],
        files: &["tests/al-language/testpage/lifecycle/ReopenAfterListModeSwitch.al"],
        since: "2026-11-20",
    },
}
