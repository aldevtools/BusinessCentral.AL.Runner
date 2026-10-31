WindowsLanguagePrimaryLanguageId => {
    id: "windows-language-primary-language-id",
    assumed: "Windows Language.\"Primary Language ID\" answers the numbers writer Y11 measured on real BC for 83 LCIDs (a culture family's LCID: 3079 -> 1031, 2052 -> 1028, 9242 -> 1050); any other row leaves the column unknown. The pinned corpus has only that two English cultures share it and German does not (record/TestWindowsLanguageTable.al:70-73).",
    tests: "No Microsoft test in the runner's suites: an outside report (two tests of a partner suite map the user's language to a culture through it; Subscription Billing's Field Translation passes it to Language.GetLanguageCode).",
    probe: "Pinned: record/lt-y11-windows-language/Y11WindowsLanguagePrimary.al is in the corpus, but 6 of its 7 tests read rows the runtime does not serve (7177, 6 and others are not rows of AgWindowsLanguage.al): serve those rows, compare every value with the file, then retire.",
}
