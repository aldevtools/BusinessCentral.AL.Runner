TernaryMaxLengthLabel => {
    id: "ternary-maxlength-label",
    assumed: "A conditional expression with a Label that has MaxLength against a Text or Code branch (sized or not, either order) behaves as with a Label without MaxLength: the true branch decides the type (real BC values that corpus writer Y66 measured, not on corpus master yet).",
    tests: "No Microsoft test: al-runner issue 6659, `(Rec.Field <> '') ? Rec.Field : DefaultLbl` with `DefaultLbl: Label '...', MaxLength = 100`.",
    probe: "Y66TernaryLabel.al:65-1335: Text[100], Code[10], Text[3], Code[250] and Text against a MaxLength label either way, into Text, Text[100], Text[5], Code[20] and a Variant.",
    awaiting: Awaiting {
        issues: &[6659],
        files: &["tests/al-language/types/lt-y66-ternary-label/Y66TernaryLabel.al"],
        since: "2026-11-01",
    },
}
