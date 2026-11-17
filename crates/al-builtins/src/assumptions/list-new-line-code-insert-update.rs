ListNewLineCodeInsertUpdate => {
    id: "list-new-line-code-insert-update",
    assumed: "A List page without parts (no DelayedInsert, no OnFindRecord or OnNextRecord, a stored source; OnAfterGetRecord and OnAfterGetCurrRecord both declared, or neither) opened for edit stands on the line New() started; a SetValue runs the table field's OnValidate, which inserts Rec itself, and the control's OnValidate then calls CurrPage.Update(), or CurrPage.Update(false) on the page with both row triggers. The page leaves the line for the stored row: Update() modifies it inside the OnValidate (OnModifyRecord and the table's OnModify, no OnInsertRecord, no second insert), Update(false) saves nothing; the refresh reads the view from its first row, the row again and OnAfterGetCurrRecord; the next SetValue lands on the row, and Previous() on the view's only row answers false without a save. Measured on Business Central 29.1 with and without AutoSplitKey, opened with OpenEdit and with Page.RunModal, on an empty view and with a row before. Every other page (one with parts among them: its FactBoxes read at other steps than the runner models), mode and line keeps the refusal.",
    tests: "No Microsoft test: al-runner issue 8186 (a partner app's test whose modal attribute list inserts its line from the table field's OnValidate; the reporter's repro is green on real BC).",
    probe: "Corpus pull request 1425 (merged, not pinned yet), tests/al-language/pages/currpage/ListNewLineCodeInsertUpdate.al:515-610 (pages with both row triggers) and :783-810 (no page trigger), green on every supported major in its CI; :713-781 is the host with CardPart FactBoxes, which the runner does not pass at its open and New() steps.",
    awaiting: Awaiting {
        issues: &[8186],
        files: &["tests/al-language/pages/currpage/ListNewLineCodeInsertUpdate.al"],
        since: "2026-11-16",
    },
}
