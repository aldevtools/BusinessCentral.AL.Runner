CurrPageSaveRecordNameCollision => {
    id: "curr-page-save-record-name-collision",
    assumed: "CurrPage.SaveRecord calls the platform method when the current page declares an internal procedure named SaveRecord.",
    tests: "AL Runner issue 7887; corpus pull request 1349.",
    probe: "Corpus pull request 1349.",
    awaiting: Awaiting {
        issues: &[7887],
        files: &["tests/al-language/pages/lifecycle/TestCurrPageSaveRecordName.al"],
        since: "2026-11-12",
    },
}
