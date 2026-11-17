MonthWeekdayNamesByLanguage => {
    id: "month-weekday-names-by-language",
    assumed: "The format codes <Month Text> and <Weekday Text> name March (1030 marts, 1036 mars, 1044 mars, 1053 mars), August (1030 august, 1036 août, 1044 august, 1053 augusti), Friday (1030 fredag, 1036 vendredi, 1044 fredag, 1053 fredag, 1031 Freitag) and Tuesday (1030 tirsdag, 1036 mardi, 1044 tirsdag, 1053 tisdag, 1031 Dienstag); <Month Text,3> and <Weekday Text,2> cut the name to that many characters. Other months, weekdays and widths in these languages refuse.",
    tests: "Outside reporter's measurement on a W1 BC of the second supported major, en-US region (al-runner issue 8183): Format(D, 0, picture) for 15 March 2024 and 6 August 2024 with <Month Text>, <Month Text,3>, <Weekday Text>, <Weekday Text,2> and <Day,2>. <Month Text> <Year4>.",
    probe: "Corpus PR 1428 (open, observe form), tests/al-language/localization/global-language/GlobalLanguageMonthWeekday.al: <Month Text> and <Weekday Text>, with no width and widths 2, 3 and 10, for every month and weekday under 1030, 1031, 1036, 1044, 1053.",
    awaiting: Awaiting {
        issues: &[8183],
        files: &["tests/al-language/localization/global-language/GlobalLanguageMonthWeekday.al"],
        since: "2026-11-17",
    },
}
