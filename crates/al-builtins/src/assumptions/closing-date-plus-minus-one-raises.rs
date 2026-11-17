ClosingDatePlusMinusOneRaises => {
    id: "closing-date-plus-minus-one-raises",
    assumed: "Under GlobalLanguage 1033, a closing date of a defined day plus the Integer 1, and minus the Integer 1, raises `The operation cannot be performed on a closing date value.` with code ClosingDateOperation (the text and code the corpus pins for a for loop over a closing date). Another number, another number type, a number before the closing date and another language refuse.",
    tests: "Corpus PR 1428 (open, commit 04e1fa0a), measured on a local BC of each of the three supported majors, the same on each: tests/al-language/localization/global-language/GlobalLanguageClosingDate.al:331-359 (ClosingDate_Arithmetic: `CD + 1` at :349 and `CD - 1` at :350 raise, code ClosingDateOperation).",
    probe: "Corpus PR 1428 (open), tests/al-language/localization/global-language/GlobalLanguageClosingDate.al. Not measured: another number, a Decimal or BigInteger, the number first, another language.",
    awaiting: Awaiting {
        issues: &[8199],
        files: &["tests/al-language/localization/global-language/GlobalLanguageClosingDate.al"],
        since: "2026-11-17",
    },
}
