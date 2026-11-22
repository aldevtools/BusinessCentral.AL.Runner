FieldRefValueTemporalError => {
    id: "fieldref-value-temporal-error",
    assumed: "On every supported BC major, assigning FieldRef.Value() to a DateTime, Date or Time variable raises an error and leaves the variable as it was, for the measured cells only. The earliest supported major gives a Conversion error naming the .NET types, later majors the Evaluate:Conversion error of the value's text (a Date names its device date settings). Cells: an Integer or BigInteger field into a var parameter of each target; into a DateTime in a var parameter, a plain local of a procedure, a by-value parameter and a named return value, the Integer, BigInteger, Decimal and Duration values and the Boolean the probe lists and the ten digit strings of a Text or Code field; into a Date or Time in a var parameter and a plain local of a procedure, the Integer, Decimal and Duration values and the ten digit strings the probe lists. The Decimal, Duration and Boolean text is the en-US text (1,000.5, 1 minute 5 seconds, Yes), so the cells run only in language 1033 with no format region.",
    tests: "Corpus PR https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/pull/1511 (merged, not yet pinned), recordref-fieldref/fieldref-value-convert/FvcIntegerField.al, FvcBigIntegerField.al and FvcOtherFields.al (a var parameter in a TryFunction), and open corpus PR https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/pull/1522, FvcDateTimeShapes.al, FvcDateTimeShapesDateTime.al and FvcDateTimeDigits.al (every cell inside a TryFunction), measured on a local BC of each supported major.",
    probe: "Pin corpus PRs 1511 and 1522 with the next corpus bump. Not measured, so still refused: a global, a field, an array element, an argument, an exit value, a local of a trigger, any value outside the probe's lists, a Date or Time in a by-value parameter or a named return value, a BigInteger or Boolean into a Date or Time other than a BigInteger var parameter, any other digit string, and any other format region or language.",
    awaiting: Awaiting {
        issues: &[8662],
        files: &[
            "tests/al-language/recordref-fieldref/fieldref-value-convert/FvcIntegerField.al",
            "tests/al-language/recordref-fieldref/fieldref-value-convert/FvcBigIntegerField.al",
            "tests/al-language/recordref-fieldref/fieldref-value-convert/FvcOtherFields.al",
            "tests/al-language/recordref-fieldref/fieldref-value-convert/FvcDateTimeShapes.al",
            "tests/al-language/recordref-fieldref/fieldref-value-convert/FvcDateTimeShapesDateTime.al",
            "tests/al-language/recordref-fieldref/fieldref-value-convert/FvcDateTimeDigits.al",
        ],
        since: "2026-11-22",
    },
}
