WindowsLanguageReportedRows => {
    id: "windows-language-reported-rows",
    assumed: "Windows Language rows 2107 (Sami, Northern (Sweden), SMF, primary 1083, code page 850, tag se-SE) and 9225 (English (Caribbean), ENB, primary 1033, code page 850, tag en-029) exist and are globally enabled, with the values the reporter measured on the newest major.",
    tests: "No Microsoft test: an outside report, the reporter's run on an on-prem container of the newest major (W1, Windows): a partner suite's install checks a fixed list of LCIDs with SetRange(\"Globally Enabled\", true) and IsEmpty.",
    probe: "Corpus writer Y39 pins every row and column of the table per major (record/lt-y39-windows-language-table/Y39WindowsLanguageTable.al, corpus pin 92031e20), but its row tests are not run here (the rows the Linux containers report are held back as Suspect_); serve the rows from it and retire this entry.",
}
