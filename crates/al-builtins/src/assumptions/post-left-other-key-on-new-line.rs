PostLeftOtherKeyOnNewLine => {
    id: "post-left-other-key-on-new-line",
    assumed: "A Worksheet with the new-row line (AutoSplitKey, DelayedInsert), filtered to one batch in filter group 2, whose action deleted its only row and left Rec, with CurrPage.Update(false), on a primary key no row has, runs as when the action only deleted the row (PnGRefresh.al:702): OnAfterGetRecord of the deleted row, then OnNewRecord and OnAfterGetCurrRecord on the blank line three times (`R:10000;N:0;C:0;N:0;C:0;N:0;C:0;`). Built only for the reporter's page and one row shown before.",
    tests: "No Microsoft test: al-runner issue 7821 (the reporter's real-BC run of NewRowPageOpenedWithPageRun and NewRowPageOpenedWithOpenEdit on real BC).",
    probe: "testpage/R7821JournalPostNewLine.al: the repro's page with one, two and three rows, opened with OpenEdit and with Page.Run, on every supported major.",
    awaiting: Awaiting {
        issues: &[7821],
        files: &["tests/al-language/testpage/R7821JournalPostNewLine.al"],
        since: "2026-11-12",
    },
}
