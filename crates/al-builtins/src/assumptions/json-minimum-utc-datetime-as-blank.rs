JsonMinimumUtcDateTimeAsBlank => {
    id: "json-minimum-utc-datetime-as-blank",
    assumed: "JsonValue.AsDateTime reads the exact string 0001-01-01T00:00:00Z as 0DT.",
    tests: "al-runner issue 7907 and corpus PR #1361, record/crud/TestMinimumUtcDateTimeStorage.al (BC 27.5, 28.5 and 29.0).",
    probe: "Corpus PR #1361 checks temporary and stored DateTime values.",
    awaiting: Awaiting {
        issues: &[7907],
        files: &["tests/al-language/record/crud/TestMinimumUtcDateTimeStorage.al"],
        since: "2026-11-13",
    },
}
