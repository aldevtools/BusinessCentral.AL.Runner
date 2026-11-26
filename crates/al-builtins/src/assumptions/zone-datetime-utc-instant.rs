ZoneDatetimeUtcInstant => {
    id: "zone-datetime-utc-instant",
    assumed: "TimeZoneInfo.GetUtcOffset(DateTime) of a zone the host holds reads an AL DateTime that is a by-value DateTime parameter of the running AL procedure as a UTC moment: .NET gets it with kind Utc. The kind does not depend on the zone or the year; the zone data is the host's, as on main (off Windows the policy still limits GetUtcOffset to the session clock's year). A DateTime of another origin (a local or global variable, a record field, the result of an AL procedure, CreateDateTime, CurrentDateTime, a DotNet DateTime, any other expression), a Date, IsDaylightSavingTime (measured on BC on Linux only) and every other TimeZoneInfo member with a DateTime or Date argument are refused.",
    tests: "No Microsoft test: al-runner issue 8587 (the reporter's run on a sandbox of major 29: System App Time Zone.GetTimezoneOffset for W. Europe Standard Time at eight moments of 2023, the transitions included). The probe's observe run (corpus pull request 1582, observe form) passes the DateTime as a procedure parameter only (`Run(ZoneId, Dt, Member)`, `DoMember`); that run gave the same values on the corpus CI run of each of majors 27, 28 and 29. No other origin was measured.",
    probe: "tests/al-language-onprem-dotnet/dotnet/timezone-info/DotNetTimeZoneDst.al (corpus pull request 1582): an AL DateTime in a DotNet DateTime has kind Utc; GetUtcOffset and IsDaylightSavingTime of an AL DateTime switch at the UTC moment of each transition of W. Europe (2023 and the current year), AUS Eastern and Pacific Standard Time (2023), one second before and after included. The DateTime is a parameter in every cell.",
    awaiting: Awaiting {
        issues: &[8587],
        files: &["tests/al-language-onprem-dotnet/dotnet/timezone-info/DotNetTimeZoneDst.al"],
        since: "2026-11-25",
    },
}
