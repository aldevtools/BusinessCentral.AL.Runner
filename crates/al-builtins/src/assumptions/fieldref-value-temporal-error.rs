FieldRefValueTemporalError => {
    id: "fieldref-value-temporal-error",
    assumed: "On every supported BC major, assigning FieldRef.Value() of an Integer or BigInteger field to a local Date, Time or DateTime variable or var parameter, or of a Decimal (with a fraction, below 1000), Duration (above 0) or Boolean field to a local DateTime, raises an error and leaves the variable as it was: the earliest supported major gives a Conversion error naming the .NET types, later majors the Evaluate:Conversion error of the value's text (the Integer and BigInteger digits without grouping).",
    tests: "Corpus PR https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/pull/1511 (merged, not yet pinned), recordref-fieldref/fieldref-value-convert/FvcIntegerField.al, FvcBigIntegerField.al and FvcOtherFields.al, measured on a local BC of each supported major and in the corpus CI. The probe assigns to a var parameter; a plain local is the al-runner issue's shape.",
    probe: "Pin corpus PR 1511 with the next corpus bump. Not measured, so still refused: a global, a field, an array element, an argument, an exit value, a whole Decimal or one of 1000 or more, a Duration of 0 or less, and a Decimal, Duration or Boolean into a Date or Time.",
    awaiting: Awaiting {
        issues: &[8662],
        files: &[
            "tests/al-language/recordref-fieldref/fieldref-value-convert/FvcIntegerField.al",
            "tests/al-language/recordref-fieldref/fieldref-value-convert/FvcBigIntegerField.al",
            "tests/al-language/recordref-fieldref/fieldref-value-convert/FvcOtherFields.al",
        ],
        since: "2026-11-22",
    },
}
