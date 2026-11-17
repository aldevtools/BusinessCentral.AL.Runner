BooleanFilterWordsDanish => {
    id: "boolean-filter-words-danish",
    assumed: "Record.SetFilter in AL code on a Boolean field under GlobalLanguage 1030, with no placeholder argument and the whole filter text as written: `Ja` filters to true, `Nej` to false, and `Yes` raises `Filteret \"Yes\" er ikke gyldigt for feltet <field caption> i tabellen <table caption>. Værdien \"Yes\" kan ikke evalueres til typen Boolean.` (code Filter:CSide). Every other text under 1030 (another case, a blank around the word, operators, alternatives, placeholders), FieldRef.SetFilter and SetView keep main's refusal.",
    tests: "Corpus PR 1428 (open), measured on a local BC of each of the three supported majors, the same on each: tests/al-language/record/filters/FilterGlobalLanguageDateBool.al:375-429 (Ja, ja, Nej, Ja|Nej, true, False, 1, 0 select rows; Yes, No, Oui, Sand, Nei raise the Danish text, code Filter:CSide). Before that: Outside reporter's measurement on a W1 BC of the second supported major, en-US region (al-runner issue 8197, shape 3): SetFilter(Active, 'Ja') and 'Nej' select the true and false rows; SetFilter(Active, 'Yes') raises the Danish text.",
    probe: "Corpus PR 1428 (open), tests/al-language/record/filters/FilterGlobalLanguageDateBool.al. Not measured: FieldRef.SetFilter, SetView, blanks around a word.",
    awaiting: Awaiting {
        issues: &[8197],
        files: &["tests/al-language/record/filters/FilterGlobalLanguageDateBool.al"],
        since: "2026-11-17",
    },
}
