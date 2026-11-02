ActionInsertedRowsHeld => {
    id: "action-inserted-rows-held",
    assumed: "A row stored while a list is open: after an action the client holds the rows of the view as they are now (a row the action inserted is found by GoToRecord, with or without CurrPage.Update), held without a read; a row the test stored before the first row held is found by the walk of a list without OnAfterGetCurrRecord, and missed with TestRowNotFound on a list with OnAfterGetCurrRecord only, the page staying on its row. Which rows the client reads on the way is not pinned.",
    tests: "No Microsoft test: al-runner issue 6671, the reporter's Worksheet with empty OnAfterGetRecord and OnAfterGetCurrRecord whose action inserts a row before the current one.",
    probe: "Measured on a real SaaS sandbox by corpus writer Y65 (branch al-runtime-rs-staging-y65 eefaf01b, corpus PR to master open).",
    awaiting: Awaiting {
        issues: &[6671],
        files: &["tests/al-language/pages/lt-y65-gotorecord-action-insert/Y65GoToRecordActionInsert.al"],
        since: "2026-11-01",
    },
}
