ZoneDatetimeUtcInstant => {
    id: "zone-datetime-utc-instant",
    assumed: "TimeZoneInfo.GetUtcOffset(DateTime) and TimeZoneInfo.IsDaylightSavingTime(DateTime) of a zone the host holds read an AL DateTime (a DateTime variable, field or parameter, the result of an AL procedure that returns a DateTime, or of CreateDateTime or CurrentDateTime) as a UTC moment: .NET gets it with kind Utc. A DateTime of another origin, a Date, and every other TimeZoneInfo member with a DateTime or Date argument are refused.",
    tests: "No Microsoft test: al-runner issue 8587 (the reporter's run on a BC 29.1 sandbox: System App Time Zone.GetTimezoneOffset for W. Europe Standard Time at eight moments of 2023, the transitions included). The probe's observe run on corpus CI gave the same values on BC 27.5, 28.5 and 29.0.",
    probe: "tests/al-language-onprem-dotnet/dotnet/timezone-info/DotNetTimeZoneDst.al (corpus pull request 1582): an AL DateTime in a DotNet DateTime has kind Utc; GetUtcOffset and IsDaylightSavingTime of an AL DateTime switch at the UTC moment of each transition of W. Europe (2023 and the current year), AUS Eastern and Pacific Standard Time (2023), one second before and after included.",
    awaiting: Awaiting {
        issues: &[8587],
        files: &["tests/al-language-onprem-dotnet/dotnet/timezone-info/DotNetTimeZoneDst.al"],
        since: "2026-11-25",
    },
}
