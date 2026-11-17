CalcdateLettersByLanguage => {
    id: "calcdate-letters-by-language",
    assumed: "CalcDate(Text, Date) under GlobalLanguage 1030, 1031, 1036, 1044 and 1053, only the measured texts. The text 1D without angle brackets reads the language's letters: 1030, 1044 and 1053 give the next day, 1031 and 1036 raise the language's should-include-a-time-unit error that lists the local units (code Evaluate). A formula <1X> in angle brackets, X one of U, K, Å, T, J, Z, reads English letters and raises the language's error that lists the English units (code Evaluate). Every other text without brackets that has a letter refuses. A text without a letter, every other bracketed formula, and every language but these five keep main's English reading; its should-include-a-time-unit error refuses under every language but 1033.",
    tests: "Corpus PR 1428 (open, commit 04e1fa0a), measured on a local BC of each of the three supported majors, the same on each: tests/al-language/localization/global-language/GlobalLanguageDateFormula.al:540-629 (CalcDate_Units_*: 1D without brackets), :642-701 (CalcDate_LocalLetters1_*: <1U>, <1K>, <1Å>), :714-771 (CalcDate_LocalLetters2_*: <1T>, <1J>, <1Z>).",
    probe: "Corpus PR 1428 (open), tests/al-language/localization/global-language/GlobalLanguageDateFormula.al. Not measured: other texts without brackets (1W, 1M, CM), two terms with a local letter, the statement form outside a TryFunction.",
    awaiting: Awaiting {
        issues: &[8200],
        files: &["tests/al-language/localization/global-language/GlobalLanguageDateFormula.al"],
        since: "2026-11-17",
    },
}
