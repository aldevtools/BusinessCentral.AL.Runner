DatasetDecimalKeepsItsZeros => {
    id: "dataset-decimal-keeps-its-zeros",
    assumed: "The report dataset writes a Decimal variable or field named as the column's source with the trailing zeros the value holds (`Round(3, 0.25)` is `3.00`, the literal 2.50 is `2.50`), with or without the column's own DecimalPlaces, and writes zero without decimals (the literal 0.00 is `0`). Pinned: one and minus one without decimals in every source shape, and an expression column with the scale of the calculation: reports/dataset/DatasetDecimalPlacesValueOrigins.al:241-247, :364, :397, :425-570, reports/layouts-saveas/TestReportSaveAsColumnTypesContracts.al:309.",
    tests: "No Microsoft test. Measured on BC 27.5, 28.5 and 29.0 by the CI of corpus pull requests 1337 and 1377, and by an outside reporter's repro on BC 29.1 (al-runner issue 7958: a Price column with DecimalPlaces = 0 : 5 over a variable set from Round holds `2.50`, `3.00`, `2.50`, `1.20000`).",
    probe: "Corpus pull request 1377 (reports/dataset/DatasetDecimalRoundScaleSources.al: a global Decimal variable as the column source by name, with and without own DecimalPlaces, set from Round, literals, sums, products and Evaluate; zero, one and minus one).",
    awaiting: Awaiting {
        issues: &[7440, 7958],
        files: &[
            "tests/al-language/reports/dataset/DatasetDecimalRoundScale.al",
            "tests/al-language/reports/dataset/DatasetDecimalRoundScaleSources.al",
        ],
        since: "2026-11-13",
    },
}