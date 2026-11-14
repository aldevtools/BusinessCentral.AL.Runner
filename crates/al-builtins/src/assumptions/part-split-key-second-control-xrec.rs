PartSplitKeySecondControlXrec => {
    id: "part-split-key-second-control-xrec",
    assumed: "A new line of an AutoSplitKey ListPart without DelayedInsert on a Card host the test opened with OpenEdit, the pages and calls of PartSplitKeyOtherSortKey.al (`auto_split_part_pinned`), read in the primary key or in a secondary key of the SubPageView, ascending, and started with New() below a stored row that is the last row of the part's view: when SetValue moves the focus to a second control and the line is inserted, the part's OnInsertRecord sees the row the line was started below as xRec (main gave the blank row). Kept as before: a line started below a row in the middle of the view (already that row), a part on no row (blank xRec), a part without OnInsertRecord, and every other host, open mode, order or call sequence.",
    tests: "None. Outside report: al-runner issue 8050.",
    probe: "Written and measured: open corpus PR 1399, tests/al-language/pages/parts/PartSplitKeyInsertTiming.al (codeunit 62733) :552 Timing_Card_Two_DescrSeq, :582 _SeqDescr, :612 _QtyDescr, :644 _Three (`INS 35000 x=20000`), :921 Timing_CardPK_Two and :953 _Three (`INS 50000 x=40000`); the same on BC 27.5, 28.5 and 29.0 in its CI run on 6cd5b3b6. Wanted: a descending order and a part with its own SourceTableView.",
    awaiting: Awaiting {
        issues: &[8050],
        files: &["tests/al-language/pages/parts/PartSplitKeyInsertTiming.al"],
        since: "2026-11-14",
    },
}
