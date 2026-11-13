RoundKeepsThePrecisionDecimals => {
    id: "round-keeps-the-precision-decimals",
    assumed: "A `Round` result keeps the decimals of the precision as written, zeros included (`Round(2.5, 0.01)` is `2.50`, `Round(3, 0.25)` is `3.00`, `Round(2, 0.01)` is `2.00`, `Round(1.200001, 0.00001)` is `1.20000`, `Round(1.4, 0.50)` is `1.50`). Pinned only for a result that needs every digit (`Round(1.234567, 0.00001)` is `1.23457`) and for zero, one and minus one, whose zeros the dataset never shows: reports/dataset/DatasetDecimalPlacesValueOrigins.al:425-450.",
    tests: "No Microsoft test. Measured on BC 27.5, 28.5 and 29.0 by the CI of corpus pull requests 1337 (run 43173667684) and 1377, and by an outside reporter's repro on BC 29.1 (al-runner issue 7958: a Price column over `Round(2.5, 0.01)`, `Round(3, 0.25)`, `Round(2.50 * 1, 0.01)` and `Round(1.200001, 0.00001)` holds `2.50`, `3.00`, `2.50`, `1.20000`).",
    probe: "Corpus pull requests 1337 (reports/dataset/DatasetDecimalRoundScale.al: Round of a number with fewer, as many and more decimals than the precision, a quarter precision, the directions, a negative number) and 1377 (reports/dataset/DatasetDecimalRoundScaleSources.al: a whole number with and without decimals, a precision written with a trailing zero, the repro of al-runner issue 7958).",
    awaiting: Awaiting {
        issues: &[7440, 7958],
        files: &[
            "tests/al-language/reports/dataset/DatasetDecimalRoundScale.al",
            "tests/al-language/reports/dataset/DatasetDecimalRoundScaleSources.al",
        ],
        since: "2026-11-12",
    },
}