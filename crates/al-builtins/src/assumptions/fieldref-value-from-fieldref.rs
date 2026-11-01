FieldRefValueFromFieldRef => {
    id: "fieldref-value-from-fieldref",
    assumed: "`FieldRef.Validate(OtherFieldRef)` and `FieldRef.Value := OtherFieldRef` take the source field's value and convert it as that value would be converted (Text to Code in upper case, numbers, the Evaluate errors); Validate runs the target's OnValidate, Value does not. Measured on real BC by corpus writer Y47.",
    tests: "No Microsoft test: al-runner issue 6591 (23 tests of the reporter's app copy mapped fields between records this way).",
    probe: "Validate and Value of a Text, Code, Decimal, Integer, Date, Boolean, Option and Enum target from a FieldRef of each other type: the stored value, the OnValidate count, the conversion errors.",
    awaiting: Awaiting {
        issues: &[6591],
        files: &["tests/al-language/recordref/lt-y47-validate-fieldref/Y47ValidateFieldRef.al"],
        since: "2026-11-01",
    },
}
