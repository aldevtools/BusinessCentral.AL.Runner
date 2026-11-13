PostLeftOtherKeyNewLineVariants => {
    id: "post-left-other-key-new-line-variants",
    assumed: "The journal of al-runner issue 7821 (a Worksheet with the new-row line, filtered to batch B1 in filter group 2, whose Post action deletes the batch's rows and leaves Rec, with CurrPage.Update(false), on Line No. 1, under any object and variable names) runs OnAfterGetRecord of the first deleted row, then OnNewRecord and OnAfterGetCurrRecord on the blank line three times, also in these cells, each the only call of the test before Close but value reads: the page with the field Batch Name first in the repeater and one row (Page.Run of the filtered record on a trapped TestPage, or OpenEdit); two rows (five blank-line pairs, four on the major of Gate::ThreeLineLeavesKeyOnceFrom28) or three rows (five pairs on every major), Page.Run; one more row, Line No. 10000 and Quantity 100, in batch B2 (Page.Run, OpenEdit) or A1 (Page.Run); the two manual subscriber codeunits to OnBeforeActionEvent and OnAfterActionEvent of Post declared, none bound or one instance of either bound (counting: Page.Run, OpenEdit; logging: Page.Run, which logs BA and AA after OnAfterGetRecord and before the blank line). After the Post the page shows line 0 and batch B1, and Close runs no trigger. Every other combination keeps main's refusal.",
    tests: "No Microsoft test. The outside reporter's own run on real BC of variants V1b, V2, V3 and V4 (al-runner issue 7821, comment of the variant table).",
    probe: "Corpus PR 1384 (merged on corpus master, not pinned yet), testpage/R7821JournalPostVariants.al codeunit 67210: all 11 tests pass on every supported major.",
    awaiting: Awaiting {
        issues: &[7821],
        files: &["tests/al-language/testpage/R7821JournalPostVariants.al"],
        since: "2026-11-13",
    },
}
