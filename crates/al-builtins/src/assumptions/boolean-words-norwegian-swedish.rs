BooleanWordsNorwegianSwedish => {
    id: "boolean-words-norwegian-swedish",
    assumed: "Under GlobalLanguage 1044 and 1053, Format(true) with standard format 0 gives Ja; Evaluate(Boolean, Text) without a format number, with the result used, reads the texts as written Ja, true and 1 as true, false and Nei (1044) or Nej (1053) as false, and rejects Yes. Format(false), Format with standard format 1, another case, a blank around the word, every other text, and the error of the statement form refuse.",
    tests: "Outside reporter's measurement on a W1 BC of the second supported major, en-US region (al-runner issue 8182): Format(true) and Format(true, 6) give Ja / `Ja    `; Evaluate of Ja, Nei (1044), Nej (1053) succeeds; Evaluate of Yes fails; true, false and 1 read in all five measured languages.",
    probe: "Corpus PR 1428 (open, observe form), tests/al-language/localization/global-language/GlobalLanguageBoolean.al: Format(true), Format(false), Evaluate of Ja, Nei, Nej, Yes, No, true, false, 1, 0, N, Y under 1044 and 1053.",
    awaiting: Awaiting {
        issues: &[8182],
        files: &["tests/al-language/localization/global-language/GlobalLanguageBoolean.al"],
        since: "2026-11-17",
    },
}
