ListNewLineWritesOtherTable => {
    id: "list-new-line-writes-other-table",
    assumed: "On the page and in the opens of `list-new-line-extra-code` under a Page.RunModal statement on one group, when the table field's OnValidate inserted the new line as the store's first write since the line began and the page's table's only one, and the table field or the control's OnValidate wrote other tables besides before CurrPage.SaveRecord() or CurrPage.Update(true) (inserts and modifies, no delete anywhere): the save and CurrPage.Update(false) do what they do without those writes, and the other tables keep them; also when the control's OnValidate then wrote the line once through another record before the save (on an empty group), which the save writes Rec over. A write before the insert, a delete, Update(true) with a row before the line, and every other open keep the earlier behavior.",
    tests: "No Microsoft test: al-runner issue 8735 (the reporter's repro, 1 test, green on real Business Central 29.1, 3/3).",
    probe: "Corpus pull request 1527 (open), tests/al-language/pages/currpage/ListNewLineWritesOtherTable.al: the objects :19-315 and the cells :328-474, 11 tests (inserts of another table by the table field :328 and :342 and by the control :361 and :373, before Update(true) :386, modifies by the table field :398 and :411 and by the control :425, before the rewritten row :463; a write before the insert :438 and a delete :450, refused), green on a local Business Central 29.1 container, 3 runs of 3.",
    awaiting: Awaiting {
        issues: &[8735],
        files: &["tests/al-language/pages/currpage/ListNewLineWritesOtherTable.al"],
        since: "2026-11-22",
    },
}
