ClosingDateMinusDateRaises => {
    id: "closing-date-minus-date-raises",
    assumed: "Under GlobalLanguage 1033, the closing date of a defined day minus the plain date of the same day raises `The operation cannot be performed on a closing date value.` (the text the corpus pins for a for loop over a closing date). Another day, a non-closing date minus a closing date, a closing date minus a closing date, a closing date minus 0D, 0D minus a closing date, and another language refuse.",
    tests: "Corpus PR 1428 (open), measured on a local BC of each of the three supported majors, the same on each: tests/al-language/localization/global-language/GlobalLanguageClosingDate.al:319 (CD-D, D-CD, CD+1, CD-1 raise, code ClosingDateOperation; CD-CD is 0). Before that: Outside reporter's repro on a W1 BC of the second supported major (al-runner issue 8199): asserterror N := ClosingDate(DMY2Date(31, 12, 2023)) - DMY2Date(31, 12, 2023) gives GetLastErrorText `The operation cannot be performed on a closing date value.`",
    probe: "Corpus PR 1428 (open), tests/al-language/localization/global-language/GlobalLanguageClosingDate.al. Not measured: a closing date minus 0D.",
    awaiting: Awaiting {
        issues: &[8199],
        files: &["tests/al-language/localization/global-language/GlobalLanguageClosingDate.al"],
        since: "2026-11-17",
    },
}
