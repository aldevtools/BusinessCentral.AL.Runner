SqlMinimumUtcDateTimeStoredAsBlank => {
    id: "sql-minimum-utc-datetime-stored-as-blank",
    assumed: "Storing midnight UTC on 0001-01-01 in a non-primary-key DateTime field reads back as a blank DateTime.",
    tests: "Local corpus probe in P130DateFilters.al measured an insert after parsing JSON and transferring from a temporary record on every supported BC major.",
    probe: "Pin the measured insert and compare the parsed and stored values with 0DT.",
    awaiting: Awaiting {
        issues: &[],
        files: &["tests/al-language/record/filters/P130DateFilters.al::Record_Insert_JsonMinimumUtcDateTime_ReadsBackBlank"],
        since: "2026-11-11",
    },
}
