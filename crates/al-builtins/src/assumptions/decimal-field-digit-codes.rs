DecimalFieldDigitCodes => {
    id: "decimal-field-digit-codes",
    assumed: "Format(Rec.Field, Length, FormatString) of a Decimal field with DecimalPlaces min:max (any places) or an amount field (AutoFormatType 1, from the second supported major) and a format string of the digit codes `<Integer>`, `<Integer,N>`, `<Integer Thousand>`, `<Decimals>`, `<Decimal>`, `<Decimal,N>`, `<Sign>` and `<Filler Character,0>` (a fixed-width code of a partner app): the value is rounded half away from zero to max places, `<Integer>` writes its whole part, `<Decimals>` and `<Decimal>` write at least min places, an overflowing code shows `*`, and `<Sign>` shows the minus of a negative value that rounds to zero. The same text for a bare `<Decimal>` on a plain Decimal.",
    tests: "No Microsoft test: al-runner issue 6901 (a partner app's fixed-width packaging code, 3 of its tests).",
    probe: "tests/al-language/types/lt-at-partner-gaps/R6901DecimalFieldFormat.al (corpus branch al-runtime-rs-staging-r6901; measured on two Windows majors and the corpus CI matrix).",
    awaiting: Awaiting {
        issues: &[6901],
        files: &["tests/al-language/types/lt-at-partner-gaps/R6901DecimalFieldFormat.al"],
        since: "2026-11-03",
    },
}
