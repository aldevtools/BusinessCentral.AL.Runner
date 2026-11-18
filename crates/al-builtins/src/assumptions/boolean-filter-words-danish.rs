BooleanFilterWordsDanish => {
    id: "boolean-filter-words-danish",
    assumed: "Record.SetFilter called in AL code with the field and the filter text only (no placeholder argument), on a record that is not temporary, in filter group 0, on a Boolean field under GlobalLanguage 1030, the whole filter text as written: `Ja` filters to true, `Nej` to false, and `Yes` raises `Filteret \"Yes\" er ikke gyldigt for feltet <field caption> i tabellen <table caption>. Værdien \"Yes\" kan ikke evalueres til typen Boolean.` (code Filter:CSide). Every other text, caller and shape under 1030 keeps main's answer: a placeholder argument reads as the typed value, and the word texts (another case, a blank around the word, operators, alternatives) refuse.",
    tests: "Corpus PR 1428 (open, commit 04e1fa0a), measured on a local BC of each of the three supported majors, the same on each: tests/al-language/record/filters/FilterGlobalLanguageDateBool.al:40-46 (the call shape), :375-391 (Ja, ja, Nej, Ja|Nej, true, False, 1, 0 select rows), :429-441 (Yes, No, Oui, Sand raise the Danish text, code Filter:CSide). Before that: Outside reporter's measurement on a W1 BC of the second supported major, en-US region (al-runner issue 8197, shape 3): SetFilter(Active, 'Ja') and 'Nej' select the true and false rows; SetFilter(Active, 'Yes') raises the Danish text.",
    probe: "Corpus PR 1428 (open), tests/al-language/record/filters/FilterGlobalLanguageDateBool.al. Not measured: FieldRef.SetFilter, SetView, placeholders, a temporary record, another filter group, blanks around a word.",
    awaiting: Awaiting {
        issues: &[8197],
        files: &["tests/al-language/record/filters/FilterGlobalLanguageDateBool.al"],
        since: "2026-11-17",
    },
}
