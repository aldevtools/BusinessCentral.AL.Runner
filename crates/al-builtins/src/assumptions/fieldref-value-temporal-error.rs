FieldRefValueTemporalError => {
    id: "fieldref-value-temporal-error",
    assumed: "On every supported BC major, assigning FieldRef.Value() of an Integer or BigInteger field to a Date, Time or DateTime var parameter, or of a Decimal of exactly 1.5, a Duration of exactly 65000 ms or a Boolean field to a DateTime var parameter, raises an error and leaves the variable as it was: the earliest supported major gives a Conversion error naming the .NET types, later majors the Evaluate:Conversion error of the value's text (the Integer and BigInteger digits without grouping).",
    tests: "Corpus PR https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/pull/1511 (merged, not yet pinned), recordref-fieldref/fieldref-value-convert/FvcIntegerField.al, FvcBigIntegerField.al and FvcOtherFields.al, measured on a local BC of each supported major and in the corpus CI. The probe assigns to a var parameter inside a TryFunction only.",
    probe: "Pin corpus PR 1511 with the next corpus bump. Not measured, so still refused: a plain declared local (the issue's own shape), a by-value parameter, a named return value, a global, a field, an array element, an argument, an exit value, any Decimal but 1.5, any Duration but 65000 ms, and a Decimal, Duration or Boolean into a Date or Time. A new corpus probe PR must measure the plain local, the by-value parameter, the named return value, more Decimal and Duration values and compact date and time digit strings before they run.",
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
