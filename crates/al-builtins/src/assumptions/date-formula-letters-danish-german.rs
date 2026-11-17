DateFormulaLettersDanishGerman => {
    id: "date-formula-letters-danish-german",
    assumed: "Date formulas under GlobalLanguage 1030 and 1031, only the measured texts. Evaluate without a format number, with the result used: 1030 reads 1K as 1Q and rejects 1W, 1Q, 1Y, NU, CW, AM; 1031 reads 1T, 1W, 1M, 1J, 1Q as 1D, 1W, 1M, 1Y, 1Q and rejects 1D, 1Y, AJ, CW. Evaluate with format number 9 reads the English letters of CM+1D, 1Y+1M+1D, -CW, WD1 (1030, 1031) and 1M, -1D (1031). Format with standard format 0 shows 1Q as 1K and WD1 as UD1 (1030), and 1D, 1Y, 1Q, -1D, CM+1D, 1Y+1M+1D, -CW, WD1 as 1T, 1J, 1Q, -1T, LM+1T, 1J+1M+1T, -LW, WT1 (1031). Other texts, numbers and the statement form of the Evaluate cells refuse.",
    tests: "Corpus PR 1428 (open), measured on a local BC of each of the three supported majors, the same on each: tests/al-language/localization/global-language/GlobalLanguageDateFormula.al:198, :234, :402, :426 (Evaluate and Format under 1030 and 1031, every measured cell agrees). Before that: Outside reporter's measurement on a W1 BC of the second supported major, en-US region (al-runner issue 8200): Evaluate results and Format(DF, 0, 9) / Format(DF) for the listed texts, and Format(DF) after Evaluate(DF, text, 9) of CM+1D, 1Y+1M+1D, -CW, WD1 (1030 LM+1D, 1Å+1M+1D, -LU, UD1; 1031 LM+1T, 1J+1M+1T, -LW, WT1) and 1M, -1D (1031 1M, -1T).",
    probe: "Corpus PR 1428 (open), tests/al-language/localization/global-language/GlobalLanguageDateFormula.al. Not measured: Evaluate of a DateFormula as a statement. 1036, 1044 and 1053 are measured there too but the runtime refuses them.",
    awaiting: Awaiting {
        issues: &[8200],
        files: &["tests/al-language/localization/global-language/GlobalLanguageDateFormula.al"],
        since: "2026-11-17",
    },
}
