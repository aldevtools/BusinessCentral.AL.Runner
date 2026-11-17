BooleanFilterWordsDanish => {
    id: "boolean-filter-words-danish",
    assumed: "Record.SetFilter in AL code on a Boolean field under GlobalLanguage 1030, with no placeholder argument and the whole filter text as written: `Ja` filters to true, `Nej` to false, and `Yes` raises `Filteret \"Yes\" er ikke gyldigt for feltet <field caption> i tabellen <table caption>. Værdien \"Yes\" kan ikke evalueres til typen Boolean.` (code Filter:CSide). Every other text under 1030 (another case, a blank around the word, operators, alternatives, placeholders), FieldRef.SetFilter and SetView keep main's refusal.",
    tests: "Outside reporter's measurement on a W1 BC of the second supported major, en-US region (al-runner issue 8197, shape 3): SetFilter(Active, 'Ja') and 'Nej' select the true and false rows; SetFilter(Active, 'Yes') raises the Danish text.",
    probe: "Corpus PR 1428 (open, observe form), tests/al-language/record/filters/FilterGlobalLanguageDateBool.al: SetFilter of a Boolean field under 1030 with Ja, Nej, Yes, No, true, false, 1, 0 and a blank around Ja: rows or GetLastErrorText and GetLastErrorCode.",
    awaiting: Awaiting {
        issues: &[8197],
        files: &["tests/al-language/record/filters/FilterGlobalLanguageDateBool.al"],
        since: "2026-11-17",
    },
}
