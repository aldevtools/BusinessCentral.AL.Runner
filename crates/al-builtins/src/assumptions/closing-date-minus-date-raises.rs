ClosingDateMinusDateRaises => {
    id: "closing-date-minus-date-raises",
    assumed: "A closing date minus a defined, non-closing Date raises `The operation cannot be performed on a closing date value.` (the text the corpus pins for a for loop over a closing date). A non-closing date minus a closing date, a closing date minus a closing date, and a closing date minus 0D refuse.",
    tests: "Outside reporter's repro on BC 28.4 W1 (al-runner issue 8199): asserterror N := ClosingDate(DMY2Date(31, 12, 2023)) - DMY2Date(31, 12, 2023) gives GetLastErrorText `The operation cannot be performed on a closing date value.`",
    probe: "Corpus probe branch al-runtime-rs-staging-globallanguage-1117 (TODO: probe PR number): ClosingDate(d) - d, d - ClosingDate(d), ClosingDate(d) - ClosingDate(d), ClosingDate(d) - 0D: result or GetLastErrorText and GetLastErrorCode.",
    awaiting: Awaiting {
        issues: &[8199],
        files: &[],
        since: "2026-11-17",
    },
}
