IntegerRecordFieldInAndBoolean => {
    id: "integer-record-field-in-and-boolean",
    assumed: "Integer record-field membership in a set of integer literals followed by AND evaluates a Boolean record field or function call eagerly.",
    tests: "al-runner issue 7906 and corpus PR #1360, language/operators/TestIntegerRecordMembership.al (BC 27.5, 28.5 and 29.0).",
    probe: "Corpus PR #1360, language/operators/TestIntegerRecordMembership.al.",
    awaiting: Awaiting {
        issues: &[7906],
        files: &["tests/al-language/language/operators/TestIntegerRecordMembership.al"],
        since: "2026-11-13",
    },
}
