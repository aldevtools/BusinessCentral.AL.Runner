DateFormulaLettersDanishGerman => {
    id: "date-formula-letters-danish-german",
    assumed: "Date formula letters under GlobalLanguage 1030 and 1031 beyond the pinned ones: Evaluate without brackets reads the Danish K as Q (one number and K), and under 1031 one number with T, W, M, J or Q (T is D, J is Y); it rejects 1W, 1Q, 1Y, NU, CW, AM (1030) and 1D, 1Y, AJ, CW (1031). Format with standard format 0 writes K for Q and UD for WD under 1030, and under 1031 the German letters (T for D, J for Y, L for C, WT for WD; W, M and Q unchanged). Other texts refuse.",
    tests: "Outside reporter's measurement on BC 28.4 W1, en-US region (al-runner issue 8200): Evaluate results and Format(DF, 0, 9) / Format(DF) for the listed texts, and Format(DF) after Evaluate(DF, text, 9) of CM+1D, 1Y+1M+1D, -CW, WD1 (1030 LM+1D, 1Å+1M+1D, -LU, UD1; 1031 LM+1T, 1J+1M+1T, -LW, WT1) and 1M, -1D (1031 1M, -1T).",
    probe: "Corpus probe branch al-runtime-rs-staging-globallanguage-1117 (TODO: probe PR number): Evaluate and Format standard formats 0 and 1 of date formulas with every unit letter and WD under 1030 and 1031; CalcDate's time-unit error text under 1030 and 1031.",
    awaiting: Awaiting {
        issues: &[8200],
        files: &[],
        since: "2026-11-17",
    },
}
