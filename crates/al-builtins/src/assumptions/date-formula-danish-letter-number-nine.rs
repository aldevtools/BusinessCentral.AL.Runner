DateFormulaDanishLetterNumberNine => {
    id: "date-formula-danish-letter-number-nine",
    assumed: "Under GlobalLanguage 1030, `Ok := Evaluate(DF, '1U', 9)` and `Ok := Evaluate(DF, '1Å', 9)` (the text as written, the result used) give false and leave DF unchanged: format number 9 reads the English letters, where U and Å are no unit. Every other text with a Danish letter (U, Å, L in either case) with number 9 under 1030, and the statement form, refuse. A text in angle brackets keeps main's English reading.",
    tests: "Corpus PR 1428 (open, commit 04e1fa0a), measured on a local BC of each of the three supported majors, the same on each: tests/al-language/localization/global-language/GlobalLanguageDateFormula.al:402-422 (DateFormula_FormatAfterEvaluate9_Danish1030: `9:1U` at :418 and `9:1Å` at :420 give `false|1Å|1Y`, the formula set before kept).",
    probe: "Corpus PR 1428 (open), tests/al-language/localization/global-language/GlobalLanguageDateFormula.al. Measured, not answered: `1T` and `1J` with number 9 under 1030 (:419, :421, false; main refuses them). Not measured: other texts with U, Å or L, lower case, the statement form.",
    awaiting: Awaiting {
        issues: &[8200],
        files: &["tests/al-language/localization/global-language/GlobalLanguageDateFormula.al"],
        since: "2026-11-17",
    },
}
