RoundScaleNoWiderThanNumber => {
    id: "round-scale-no-wider-than-number",
    assumed: "`Round` drops the trailing zeros the rounding added beyond the scale of the rounded number for a number that has decimals or a precision that is not a power of ten (`Round(2.5, 0.01)` is `2.5`, `Round(3, 0.25)` is `3`). Pinned only for a whole number written without decimals and a precision of a power of ten (`Round(1, 0.00001)` is `1`) and for a result that needs every digit (`Round(1.234567, 0.00001)` is `1.23457`): reports/dataset/DatasetDecimalPlacesValueOrigins.al:425-450.",
    tests: "No Microsoft test. An outside report: a report dataset column over a Decimal field set to a rounded value is written as `1`, not `1.00000`, on real BC (al-runner issue 7440).",
    probe: "Queued as corpus PR 1337, reports/dataset/DatasetDecimalRoundScale.al: the dataset text of Round(2.5, 0.01), Round(2.50, 0.01), Round(3, 0.25), Round(1.200001, 0.00001), a variable set from the literal 2.50 times one, and a temporary row set from that literal.",
    awaiting: Awaiting {
        issues: &[7440],
        files: &["tests/al-language/reports/dataset/DatasetDecimalRoundScale.al"],
        since: "2026-11-12",
    },
}
