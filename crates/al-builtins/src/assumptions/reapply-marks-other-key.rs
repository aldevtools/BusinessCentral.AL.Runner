ReapplyMarksOtherKey => {
    id: "reapply-marks-other-key",
    assumed: "A TestPage First or GoToRecord on a list with a part, in a key other than the primary key, whose OnAfterGetCurrRecord marks rows again with MarkedOnly on and calls CurrPage.Update(false): the client holds the marked rows of the new view in that key's order and stays on the row Rec was left on, as in the primary key (pinned by pages/lt-y1-6288/Y1ExternalReapplyMarks.al:223 and :278); when Rec was left on another row than the move reached, OnAfterGetCurrRecord runs once more on it.",
    tests: "No Microsoft test: al-runner issue 6467, the reporter's run on real BC (a Worksheet in a descending SortOrder key: `before=false/1 after=false/1`). Measured by corpus writer Y15 (Y15ReapplyMarksSecondaryKey.al, on the SaaS sandbox, newest major).",
    probe: "Y15ReapplyMarksSecondaryKey.al on branch al-runtime-rs-staging-y15: First and GoToRecord after the outside refresh, the moves after them, and the OnAfterGetCurrRecord log.",
    awaiting: Awaiting {
        issues: &[6467],
        files: &["tests/al-language/pages/lt-y15/Y15ReapplyMarksSecondaryKey.al"],
        since: "2026-10-31",
    },
}
