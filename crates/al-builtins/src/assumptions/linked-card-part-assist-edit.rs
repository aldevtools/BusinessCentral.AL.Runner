LinkedCardPartAssistEdit => {
    id: "linked-card-part-assist-edit",
    assumed: "On an empty CardPart with InsertAllowed = false, no row triggers, no page extension or subscriber and a stored source table, linked from a Document to the part's single Code primary key, AssistEdit on that key receives the linked value in Rec. If the trigger inserts the matching row and returns it in Rec, a later AssistEdit and CurrPage.Update can operate on that row.",
    tests: "AL Runner issue 7883 (reporter's repro AssistEditUsesTheLinkedPrimaryKey and UpdateAfterAssistEditShowsInsertedDetail, measured on all three supported BC majors; the issue records the exact builds).",
    probe: "Corpus pull request 1347.",
    awaiting: Awaiting {
        issues: &[7883],
        files: &["tests/al-language/pages/parts/TestCardPartAssistEditKey.al"],
        since: "2026-11-12",
    },
}
