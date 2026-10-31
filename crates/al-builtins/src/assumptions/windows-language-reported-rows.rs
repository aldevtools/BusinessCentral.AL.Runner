WindowsLanguageReportedRows => {
    id: "windows-language-reported-rows",
    assumed: "Windows Language rows 2107 (Sami, Northern (Sweden), SMF, primary 1083, code page 850, tag se-SE) and 9225 (English (Caribbean), ENB, primary 1033, code page 850, tag en-029) exist and are globally enabled, with the values the reporter measured on BC 29.",
    tests: "No Microsoft test: al-runner issue 6558, the reporter's run on a BC 29.0 on-prem container (W1): a partner suite's install checks a fixed list of LCIDs with SetRange(\"Globally Enabled\", true) and IsEmpty.",
    probe: "Corpus writer Y39 pins every row and column of the table per major (record/lt-y39-windows-language-table/); serve the rows from it and retire this entry.",
    awaiting: Awaiting {
        issues: &[6558],
        files: &["tests/al-language/record/lt-y39-windows-language-table/Y39WindowsLanguageTable.al"],
        since: "2026-10-31",
    },
}
