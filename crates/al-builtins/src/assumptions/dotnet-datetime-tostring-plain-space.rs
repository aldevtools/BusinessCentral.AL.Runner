DotnetDatetimeTostringPlainSpace => {
    id: "dotnet-datetime-tostring-plain-space",
    assumed: "ToString() of a DotNet System.DateTime, written with parentheses and no argument, gives .NET's en-US text with a plain space before AM or PM, where the runner's .NET (its ICU data) writes a narrow no-break space (U+202F); every other character of .NET's answer is kept, for any moment and kind (the probe measured AM and PM, one- and two-digit hours, and kinds Utc, Local and Unspecified). Every other member and form keeps .NET's answer: a format or culture argument, ToLongTimeString, ToShortTimeString, the form without parentheses, String.Format and Convert.ToString of a DateTime, and DateTimeOffset.ToString().",
    tests: "No Microsoft test: the probe's local runs on BC on Linux of majors 27 (minor 5), 28 (minor 5) and 29 (minor 0) and its corpus CI run, with culture en-US and no user time zone.",
    probe: "tests/al-language-onprem-dotnet/dotnet/datetime-kind/DotNetDateTimeKind.al (corpus pull request 1653, codeunit 99362): the tostring step (DoStep :178) of every origin, PM at Origin_AlParam :377, Origin_CtorUnspecified :431, Origin_CtorLocal :467, Origin_AlSumTwoHoursParam :701 (2:00:00 PM), AM at Origin_ParseOffset :521 (10:00:00 AM).",
    awaiting: Awaiting {
        issues: &[9395],
        files: &["tests/al-language-onprem-dotnet/dotnet/datetime-kind/DotNetDateTimeKind.al"],
        since: "2026-11-28",
    },
}
