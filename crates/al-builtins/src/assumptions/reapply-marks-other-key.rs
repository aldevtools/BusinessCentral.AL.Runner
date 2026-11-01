ReapplyMarksOtherKey => {
    id: "reapply-marks-other-key",
    assumed: "A TestPage GoToRecord as the first move after an outside refresh, on a list with a part in a key other than the primary key whose OnAfterGetCurrRecord marks rows again with MarkedOnly on and calls CurrPage.Update(false): the client holds the marked rows of the new view in that key's order and stays on the row Rec was left on, as after First (pinned by pages/lt-y15/Y15ReapplyMarksSecondaryKey.al:269 and, in the primary key, pages/lt-y1-6288/Y1ExternalReapplyMarks.al:278).",
    tests: "No Microsoft test: an outside report (a Worksheet in a descending SortOrder key).",
    probe: "Written: pages/lt-y15/Y15ReapplyMarksSecondaryKey.al:354 Y15_SK_GoToRecordAfterRefreshWithoutFirst holds the values, but it does not run here yet (an earlier test of its codeunit is refused, Y15_SK_MovesWithoutRefresh); compare its values once it runs, then retire.",
}
