DotnetDatetimeUtcZoneConversions => {
    id: "dotnet-datetime-utc-zone-conversions",
    assumed: "ToUniversalTime(), ToLocalTime() and ToFileTime() of a DotNet System.DateTime give the same answer for every kind (Utc, Local, Unspecified), because the service tier's zone is UTC: ToUniversalTime() and ToLocalTime() keep the moment, ToFileTime() is the 100 ns intervals since 1601-01-01 of that moment. The runner answers them itself with no .NET call and no argument, as a .NET DateTime read back into AL. ToFileTime() of a moment before 1601-01-01 is refused. The kind of the result is not kept: Kind, ToBinary() and ToString with the round-trip format or a K or z format stay refused on it, as on any DotNet DateTime.",
    tests: "No Microsoft test: the probe's local runs on BC on Linux of majors 27 (minor 5), 28 (minor 5) and 29 (minor 0) (24 of 24 green on each, the same values), and its corpus CI run, all with the service tier zone UTC, no user time zone and culture en-US.",
    probe: "tests/al-language-onprem-dotnet/dotnet/datetime-kind/DotNetDateTimeKind.al (corpus pull request 1653, codeunit 99362): Host_TimeZone_Facts :254 (zone UTC, no DST); NowUtcNow_KindAndConversions :280 and Now_ToUniversalTimePlusDuration_EvaluateToString :329 (Now and UtcNow); the touniversal, tolocal and tofiletime steps of every origin, kind Utc (Origin_AlParam :367, Origin_AlLocal :385, Origin_CreateDateTime :403, Origin_CtorUtc :439, Origin_EpochAdd :547, Origin_RoundTrip* :565-:601, Origin_Variant* :637-:655, Origin_AlSum* :673-:740), kind Local (Origin_CtorLocal :457, Origin_ParseZ :475, Origin_ParseOffset :511, Origin_ParseAlFormat9 :529, Origin_DotNetCopyLocal :619) and kind Unspecified (Origin_CtorUnspecified :421, Origin_ParseNoSuffix :493).",
    awaiting: Awaiting {
        issues: &[9052, 9092],
        files: &["tests/al-language-onprem-dotnet/dotnet/datetime-kind/DotNetDateTimeKind.al"],
        since: "2026-11-28",
    },
}
