ListNewLineSaveRecordRepro => {
    id: "list-new-line-save-record-repro",
    assumed: "The pages of the reporter's repro of al-runner issue 8285, line for line (a List page with AutoSplitKey, no ApplicationArea on its controls, and either no page trigger and CurrPage.SaveRecord() then CurrPage.Update(false), or CurrPage.Update(true), in the Name control's OnValidate, or OnAfterGetRecord that counts rows and one CardPart FactBox linked on the key with SaveRecord then Update(false)), over the repro's table, opened by a Page.RunModal statement on one group with one row before the new line: they behave as the probe's pages with the same calls do (pinned by ListNewLineSaveRecordUpdate.al). The page leaves the line for the stored row, the save modifies it, the refresh lands on it, the later SetValue lands on it and OK saves it.",
    tests: "No Microsoft test: al-runner issue 8285 (the reporter's repro, 3 tests, green on real Business Central 29.1, 3/3).",
    probe: "Corpus pull request 1481 (open), tests/al-language/pages/currpage/ListNewLineSaveRecordRepro.al: the repro's objects and its 3 tests.",
    awaiting: Awaiting {
        issues: &[8285],
        files: &["tests/al-language/pages/currpage/ListNewLineSaveRecordRepro.al"],
        since: "2026-11-20",
    },
}
