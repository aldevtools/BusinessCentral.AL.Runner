PartNewOnViewHostDenied => {
    id: "part-new-on-view-host-denied",
    assumed: "New() on an AutoSplitKey ListPart (no DelayedInsert, triggers OnNewRecord and OnInsertRecord, no extension or subscriber) of a Card or Document host with no trigger that the test opened with OpenView, linked on the primary key fields but the last (an Integer) and read in a secondary key of the SubPageView, ascending, after the calls GoToKey, the part's Last, New: raises 'New method failed because Insert is not allowed. Page = <part name>, Id = <part id>.' with code DB:ClientInsertDenied and runs no trigger. Every other part New() on a host the test opened with OpenView is refused.",
    tests: "None. Outside report: al-runner issue 8050 (the area of new part rows).",
    probe: "Written and measured: open corpus PR 1399, tests/al-language/pages/parts/PartSplitKeyInsertTiming.al:1353 Doc_OpenView_New and :1385 Card_OpenView_New, the same on all three majors in its CI run on 6cd5b3b6. Wanted: a part in primary key order, a part without triggers, DelayedInsert, other calls before New().",
    awaiting: Awaiting {
        issues: &[8050],
        files: &["tests/al-language/pages/parts/PartSplitKeyInsertTiming.al"],
        since: "2026-11-14",
    },
}
