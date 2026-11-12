OpenNewTempNavigatePageEditsInsertedRow => {
    id: "open-new-temp-navigate-page-edits-inserted-row",
    assumed: "TestPage.OpenNew() of a NavigatePage over a temporary source whose only own trigger is OnOpenPage, which inserted the one row of the temporary table (`Rec.Init(); Rec.Insert();`), shows that row as OpenEdit would: no new row and no OnNewRecord, a SetValue edits the inserted row and Close saves it into the temporary table only. Only that shape: `SourceTableTemporary = true`, no page property outside Caption, Permissions, `Extensible = false`, `Editable = true`, ApplicationArea and UsageCategory, no part; a table without TableType, without OnInsert, OnModify or OnRename, without extension or subscriber; Rec unfiltered on the stored row, which holds the values of Init(). Every other page keeps the new row of OpenNew.",
    tests: "No corpus test. Email - SMTP Connector, SMTP Account Auth Tests: TestAccountNameValidation, TestOAuth2FlowWithoutCustomOAuth and the other tests that open page SMTP Account Wizard with OpenNew and SetValue (22 of 47), which pass on BC; the reporter's repro of al-runner issue 7811.",
    probe: "A NavigatePage, a Card and a List over a temporary source (and over a database table) whose OnOpenPage inserts a blank row, with and without OnNewRecord, OnAfterGetCurrRecord and OnInsertRecord logging: OpenNew, then the log, Editable(), a field's Value, SetValue on a non-key field, Close, and the rows of the temporary table seen from a page procedure; the same with OnOpenPage inserting two rows, and with Rec left on a filtered view.",
    awaiting: Awaiting {
        issues: &[7811],
        files: &["tests/al-language/pages/opennew-findrecord/OpenNewTempOnOpenPageRow.al"],
        since: "2026-11-12",
    },
}
