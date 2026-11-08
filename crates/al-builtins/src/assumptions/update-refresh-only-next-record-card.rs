UpdateRefreshOnlyNextRecordCard => {
    id: "update-refresh-only-next-record-card",
    assumed: "CurrPage.Update on a Card without unsaved edits that declares OnNextRecord and a row trigger (OnAfterGetRecord or OnAfterGetCurrRecord) but no OnFindRecord reads the current row again with the default Find('='), then runs OnAfterGetCurrRecord, as the OnFindRecord card does (update-refresh-onfindrecord). Pinned: the card stays on its row (pages/currpage/UpdateOnFindRecord.al:357, corpus pull request 1199). The trigger order of the refresh is not pinned. A list, or a card with unsaved edits, stays refused.",
    tests: "No Microsoft test: al-runner issue 7268 (the reporter's real-BC run of CurrPage_Update_CardWithOnlyOnNextRecord_KeepsCurrentRow on the three supported majors, corpus pull request 1199).",
    probe: "pages/currpage/UpdateOnFindRecord.al once merged; also wanted: the trigger log (OnNextRecord, OnAfterGetRecord, OnAfterGetCurrRecord) of that refresh.",
    awaiting: Awaiting {
        issues: &[7268],
        files: &["tests/al-language/pages/currpage/UpdateOnFindRecord.al::CurrPage_Update_CardWithOnlyOnNextRecord_KeepsCurrentRow"],
        since: "2026-11-08",
    },
}
