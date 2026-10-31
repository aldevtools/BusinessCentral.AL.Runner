FormatWeekCodeIso => {
    id: "format-week-code-iso",
    assumed: "`<Week>` and `<Week,2>` of a Date give the ISO 8601 week number (weeks start on Monday, week 1 holds the year's first Thursday) for every week, and `<Weekday>` the ISO day number, Monday 1 to Sunday 7. Pinned are the weeks 1, 3, 10, 19, 28, 40, 52 and 53 at year boundaries and mid-year (types/lt-format-week-code/FormatWeekCodeIso.al:21-123, types/lt-datetime/DateFuncs.al:415-427) and the days 3 and 7 (DateFuncs.al:415,425; FormatWeekCodeIso.al:121,128); a Format of another week or day is a flagged pass.",
    tests: "No Microsoft test: the rule generalizes the pinned weeks (review of al-runner PR 5864).",
    probe: "`<Week>`, `<Week,2>` and `<Weekday>` of one date in every ISO week of 2021 (53 dates) and of the seven days of one week.",
}
