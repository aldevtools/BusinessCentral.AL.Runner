IntegerCompoundAssignFieldrefValue => {
    id: "integer-compound-assign-fieldref-value",
    assumed: "An Integer variable `+=` or `-=` a `FieldRef.Value()` call without arguments does not compile: the call is typed Joker, and the AL compiler reports AL0175 (`Operator '+=' cannot be applied to operands of type 'Integer' and 'Joker'`, and the same for `-=`) at the statement (al-sema `check/joker_compound.rs`). `I := FR.Value()` compiles. Other left sides (BigInteger, Decimal, a field), the bare `FR.Value` and other operators are not judged and compile as before. A compile refusal ends before any test runs, so it carries no flagged-pass mark.",
    tests: "No Microsoft test. al-runner issue 9283: AL compiler 18.0.43.1464 at runtime 16.0 and 18.0, and AL compiler 18.1.43.24897 at 18.0, reject `I += FR.Value()` and `I -= FR.Value()` (I an Integer, FR a FieldRef) with AL0175 at the statement, and compile `I := FR.Value()`.",
    probe: "Corpus probe on branch probe-9283-compound-joker (its pull request is to be opened): must-not-compile 3834-integer-plus-assign-fieldref-value.al and 3835-integer-minus-assign-fieldref-value.al.",
    awaiting: Awaiting {
        issues: &[9283],
        files: &[
            "tests/must-not-compile/cases/3834-integer-plus-assign-fieldref-value.al",
            "tests/must-not-compile/cases/3835-integer-minus-assign-fieldref-value.al",
        ],
        since: "2026-11-28",
    },
}
