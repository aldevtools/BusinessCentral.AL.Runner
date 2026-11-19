DateFilterErrorByLanguage => {
    id: "date-filter-error-by-language",
    assumed: "The language's text of the date-format error of a Date filter, which names the filter as written, the field caption, the table caption and the value (code Filter:CSide), under GlobalLanguage 1030, 1036, 1044 and 1053. The runtime raises it only from the closing-letter Date filter of entry closing-date-letter-by-language, in its call shape, for the letter of another language (under 1031 that filter raises the pinned German text). Every other Date filter text no filter reads keeps main's answer: the German text under 1031, a refusal under every other language but 1033.",
    tests: "Corpus PR 1428 (open, commit 04e1fa0a), measured on a local BC of each of the three supported majors, the same on each: tests/al-language/record/filters/FilterGlobalLanguageDateBool.al:229-307 (C12/31/23 raises under 1030, 1044 and 1053), :309-372 (U12/31/23 under 1036 and 1053, S12/31/23 under 1030, 1036 and 1044), in the call shape of :40-46. The forms at :169-227 (15.03.2024, 15-03-2024.., 150324) are measured too; the runtime does not answer them.",
    probe: "Corpus PR 1428 (open), tests/al-language/record/filters/FilterGlobalLanguageDateBool.al. Not measured: FieldRef.SetFilter, SetView, a filter with an alternative or a range of two values, placeholders, a temporary record, another filter group.",
    awaiting: Awaiting {
        issues: &[8178],
        files: &["tests/al-language/record/filters/FilterGlobalLanguageDateBool.al"],
        since: "2026-11-17",
    },
}
