ListNewLineExtraCode => {
    id: "list-new-line-extra-code",
    assumed: "A List page without page triggers and parts (the measured page of ListNewLineSaveRecordUpdate.al's page 97473 kind), with AutoSplitKey, over a table without OnInsert, OnModify, OnDelete and OnRename triggers, whose new line the table field's OnValidate inserted, opened by a Page.RunModal statement on one group (empty, or with one row before the line; Update(true) on an empty group only) or, for CurrPage.SaveRecord() on an empty group, by TestPage.OpenEdit and a filter of the test on the group, whose control's OnValidate runs code that is not a pinned page's: when nothing but the insert wrote the store since the line began, CurrPage.SaveRecord() or CurrPage.Update(true) saves Rec with every field it holds (set by the control or by the table field); when the control's OnValidate also wrote the row once through another record before CurrPage.SaveRecord() (Page.RunModal on an empty group), the save writes Rec over it; between the save and CurrPage.Update(false) the code may modify the row through another record, write Rec's fields, or write other tables, but not both the row and Rec, not another row of the page's table and not Rec's view; CurrPage.Update(false) then reads the row as stored, and CurrPage.Update(true) after SaveRecord writes Rec over it again; the Update is the validation's last statement. Any other write before the save, Update() after SaveRecord, and every other page, open and view keep the earlier behavior.",
    tests: "No Microsoft test: al-runner issues 8712 and 8713 (the reporters' repros, 1 test each, green on real Business Central 29.1, 3/3).",
    probe: "Corpus pull request 1521 (open), tests/al-language/pages/currpage/ListNewLineExtraCode.al: the objects :19-329 and the cells :342-547, 14 tests (a row before the line for each effect the save and Update(false) run, :355, :414, :512, :524, :536), green on a local Business Central 29.1 container, 3 runs of 3.",
    awaiting: Awaiting {
        issues: &[8712, 8713],
        files: &["tests/al-language/pages/currpage/ListNewLineExtraCode.al"],
        since: "2026-11-22",
    },
}
