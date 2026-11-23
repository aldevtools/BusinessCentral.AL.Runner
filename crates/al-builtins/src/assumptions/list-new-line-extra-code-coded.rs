ListNewLineExtraCodeCoded => {
    id: "list-new-line-extra-code-coded",
    assumed: "A List page of the shape of ListNewLineFactboxesCode.al's page with code but not its code: AutoSplitKey, page triggers among OnOpenPage, OnAfterGetRecord and OnDeleteRecord, actions, three CardPart FactBoxes over the page's table, each linked to each field of its primary key, whose triggers are among OnAfterGetRecord, OnAfterGetCurrRecord and OnPageBackgroundTaskCompleted (one enqueues a page background task), over a table without OnInsert, OnModify, OnDelete and OnRename triggers, whose new line the table field's OnValidate inserted (and may write other tables after the insert; the control's OnValidate writes nothing before the save), opened by a Page.RunModal statement on one empty group: the open, the parts' reads and the refresh are those of the page with code (list-new-line-coded-factboxes); the control's OnValidate around CurrPage.SaveRecord() and CurrPage.Update(false) runs under the effect gate (list-new-line-extra-code), the refresh reads the row as stored and its triggers write nothing, the background task's reads go through the query cache (background-task-query-cache), and one SetValue of a control without code, then OK, save the row. Every other page, open, view and call keeps the earlier behavior.",
    tests: "No Microsoft test: al-runner issue 8826 (the reporter's repro, 1 test, green on real Business Central 29.1, 3/3).",
    probe: "Corpus pull request 1537 (open), tests/al-language/pages/currpage/ListNewLineExtraCodeCodedPage.al: the objects :21-750 and the cells :762-850, 7 tests (the repro :762; the logging page's reads and the background task at each step with the repro's code :773, only the save :785, a modify through another record after the save :797, a field set before the save :809; a row before the line :821 and a table with OnInsert and OnModify :839, refused), green on a local Business Central 29.1 container, 3 runs of 3.",
    awaiting: Awaiting {
        issues: &[8826],
        files: &["tests/al-language/pages/currpage/ListNewLineExtraCodeCodedPage.al"],
        since: "2026-11-23",
    },
}
