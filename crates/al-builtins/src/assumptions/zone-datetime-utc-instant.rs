ZoneDatetimeUtcInstant => {
    id: "zone-datetime-utc-instant",
    assumed: "TimeZoneInfo.GetUtcOffset(DateTime) and TimeZoneInfo.IsDaylightSavingTime(DateTime) of a zone the host holds read an AL DateTime (a DateTime variable, field or parameter, the result of an AL procedure that returns a DateTime, or of CreateDateTime or CurrentDateTime) as a UTC moment: .NET gets it with kind Utc. A DateTime of another origin, a Date, and every other TimeZoneInfo member with a DateTime or Date argument are refused.",
    tests: "No Microsoft test: al-runner issue 8587 (the reporter's run on a BC 29.1 sandbox: System App Time Zone.GetTimezoneOffset for W. Europe Standard Time at eight moments of 2023, the transitions included).",
    probe: "PROBE_FILE (corpus pull request PROBE_PR): the kind of an AL DateTime in .NET, GetUtcOffset and IsDaylightSavingTime at the transitions of W. Europe, AUS Eastern and Pacific Standard Time.",
    awaiting: Awaiting {
        issues: &[8587],
        files: &["PROBE_FILE"],
        since: "2026-11-25",
    },
}
