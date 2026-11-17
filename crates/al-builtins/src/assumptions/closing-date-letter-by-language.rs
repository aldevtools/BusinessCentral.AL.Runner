ClosingDateLetterByLanguage => {
    id: "closing-date-letter-by-language",
    assumed: "Format(D) with standard format 0 of a closing date marks it with the letter of GlobalLanguage: U under 1030, 1031 and 1044, S under 1053, C under 1036 (the date part as under 1033). Only standard format 0 under these five languages; standard formats 1 and 2 and the <Closing> code under any language but 1033, and standard format 0 under any other language, refuse.",
    tests: "Outside reporter's measurement on a W1 BC of the second supported major, en-US region (al-runner issue 8178 and its comment): Format(ClosingDate(DMY2Date(31, 12, 2023))) after GlobalLanguage(n) gives U12/31/23 for 1030, 1031, 1044, S12/31/23 for 1053, C12/31/23 for 1036.",
    probe: "Corpus PR 1428 (open, observe form), tests/al-language/localization/global-language/GlobalLanguageClosingDate.al: Format of a closing date under 1030, 1031, 1036, 1044, 1053 with standard formats 0, 1, 2 and the <Closing> code.",
    awaiting: Awaiting {
        issues: &[8178],
        files: &["tests/al-language/localization/global-language/GlobalLanguageClosingDate.al"],
        since: "2026-11-17",
    },
}
