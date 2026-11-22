ListNewLineExtraCodeFactbox => {
    id: "list-new-line-extra-code-factbox",
    assumed: "A List page with AutoSplitKey whose only triggers are OnAfterGetRecord and OnAfterGetCurrRecord, with one CardPart FactBox over the page's table, linked to each field of its primary key, whose only trigger is OnAfterGetRecord and whose fields show Rec's fields or a variable of its own, over a table without OnInsert, OnModify, OnDelete and OnRename triggers, whose new line the table field's OnValidate inserted, opened by a Page.RunModal statement on one group (empty, or with one row before the line), whose control's OnValidate runs code that is not a pinned page's around CurrPage.SaveRecord() and CurrPage.Update(false), with nothing but the insert written before the save: the save and the Update do what they do on the page without triggers (list-new-line-extra-code), and the refresh of the Update runs the page's OnAfterGetRecord on the row, the row before it if any, and the row twice more, then OnAfterGetCurrRecord on the row and the part's OnAfterGetRecord on the row. Every other page, open, view and write keeps the earlier behavior.",
    tests: "No Microsoft test: al-runner issue 8734 (the reporter's repro, 1 test, green on real Business Central 29.1, 3/3).",
    probe: "Corpus pull request 1532 (open), tests/al-language/pages/currpage/ListNewLineExtraCodeFactbox.al: the objects :18-307 and the cells :319-434, 10 tests (the repro :319 and :331; the logging page's reads at each step with the repro's code :344 and :355, only the save :367 and :400, a modify through another record after the save :378 and :412, a field set before the save :389 and :424), green on a local Business Central 29.1 container, 3 runs of 3.",
    awaiting: Awaiting {
        issues: &[8734],
        files: &["tests/al-language/pages/currpage/ListNewLineExtraCodeFactbox.al"],
        since: "2026-11-22",
    },
}
