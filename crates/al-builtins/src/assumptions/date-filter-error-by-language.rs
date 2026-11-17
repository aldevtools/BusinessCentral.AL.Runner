DateFilterErrorByLanguage => {
    id: "date-filter-error-by-language",
    assumed: "Record.SetFilter in AL code on a Date field under GlobalLanguage 1030, 1036, 1044 or 1053, with no placeholder argument, whose filter text is one value no Date filter reads (alone, or followed by ..): BC raises the language's text of the date-format error, which names the filter, the field caption, the table caption and the value (code Filter:CSide). The runtime raises it only for a closing date with the letter of another language (C, U or S and a date); FieldRef.SetFilter, SetView, alternatives, placeholders and every other language but 1031 and 1033 refuse.",
    tests: "Corpus PR 1428 (open), measured on a local BC of each of the three supported majors, the same on each: tests/al-language/record/filters/FilterGlobalLanguageDateBool.al:169-217 (15.03.2024, 15-03-2024.., 150324 under each language), :229-293 (C12/31/23), :300-370 (U12/31/23, S12/31/23).",
    probe: "Corpus PR 1428 (open), tests/al-language/record/filters/FilterGlobalLanguageDateBool.al. Not measured: FieldRef.SetFilter, SetView, a filter with an alternative or a range of two values, placeholders.",
    awaiting: Awaiting {
        issues: &[8178],
        files: &["tests/al-language/record/filters/FilterGlobalLanguageDateBool.al"],
        since: "2026-11-17",
    },
}
