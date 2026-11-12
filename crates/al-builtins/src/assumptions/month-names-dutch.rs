MonthNamesDutch => {
    id: "month-names-dutch",
    assumed: "Format(Date, 0, 4) and Format(Date, 0, 7) in GlobalLanguage 2067 (Dutch, Belgium) and 1043 (Dutch) write the Dutch month names: januari, februari, maart, april, mei, juni, juli, augustus, september, oktober, november, december (format 4) and jan, feb, mrt, apr, mei, jun, jul, aug, sep, okt, nov, dec (format 7, no period), in the shape `<month> <day>, <year>`. December of format 4 is pinned already (EvalLanguageBoolean.al); other languages stay refused.",
    tests: "Corpus probe branch al-runtime-rs-writer-reports6 (pull request not opened yet), format-evaluate/MonthNamesLanguages.al, observed on BC 27.5, 28.5 and 29.0: the same names in 2067 and 1043, days 5 and 15.",
    probe: "Corpus probe branch al-runtime-rs-writer-reports6 (pull request not opened yet), format-evaluate/MonthNamesLanguages.al: every month of formats 4 and 7 in 2067, 1043 and eight other languages.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/format-evaluate/MonthNamesLanguages.al"],
        since: "2026-11-12",
    },
}
