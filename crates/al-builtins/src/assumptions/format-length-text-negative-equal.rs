FormatLengthTextNegativeEqual => {
    id: "format-length-text-negative-equal",
    assumed: "Format(T, L) of a Text exactly as long as the negative length -L keeps the text. A Code, a standard format other than 0 and every other type refuse in that cell (owner al-builtins format/length.rs).",
    tests: "Corpus PR 1460 (open, commit 79c5192c), measured on a local BC of the first supported major and green in its CI on all three: tests/al-language/format-evaluate/format-length/FormatLengthTypes.al:888 (Format('abcde', -5)). Microsoft: Tests-Data Exchange codeunit Import XML Gen Jnl Line (GLAccDifferentStatementCurrency, CurrencyFactorShouldSameGLAccDifferentStatementCurrency) through Format of the 10-character Text 'GU00000015' with length -10.",
    probe: "Corpus PR 1460 (open), tests/al-language/format-evaluate/format-length/FormatLengthTypes.al. Not measured: a Code as long as a negative length, standard format 9 in that cell.",
    awaiting: Awaiting {
        issues: &[8389],
        files: &["tests/al-language/format-evaluate/format-length/FormatLengthTypes.al"],
        since: "2026-11-19",
    },
}
